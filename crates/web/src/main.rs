mod api;
mod http;

use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result};
use clap::Parser;
use lexiroot_analyzer::AnalyzerDb;
use serde_json::json;

const INDEX_HTML: &str = include_str!("../static/index.html");

/// A local test page for the analyzer. Binds to loopback by default: the
/// release database is read-only, but this server has no auth, no rate
/// limiting and no TLS, so it is not meant to face a network.
#[derive(Parser)]
#[command(name = "lexiroot-web", about = "Serve a local test page for the LexiRoot analyzer.")]
struct Cli {
    /// Path to a release SQLite database.
    #[arg(long, default_value = "data/release/lexiroot-v0.1.sqlite")]
    db: PathBuf,

    /// Address to bind. Keep it on loopback unless you know what you're doing.
    #[arg(long, default_value = "127.0.0.1")]
    host: String,

    #[arg(long, default_value_t = 8080)]
    port: u16,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let db = Arc::new(
        lexiroot_store::load(&cli.db)
            .with_context(|| format!("failed to load database at {}", cli.db.display()))?,
    );

    let listener = TcpListener::bind((cli.host.as_str(), cli.port))
        .with_context(|| format!("failed to bind {}:{}", cli.host, cli.port))?;
    println!(
        "LexiRoot test page: http://{}:{}  (db: {})",
        cli.host,
        cli.port,
        cli.db.display()
    );

    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        let db = Arc::clone(&db);
        thread::spawn(move || handle(stream, &db));
    }

    Ok(())
}

fn handle(mut stream: TcpStream, db: &AnalyzerDb) {
    let Some(request) = http::read_request(&stream) else {
        return;
    };

    if request.method != "GET" {
        http::respond_json(
            &mut stream,
            "405 Method Not Allowed",
            &json!({ "error": "only GET is supported" }),
        );
        return;
    }

    match request.path.as_str() {
        "/" | "/index.html" => http::respond(
            &mut stream,
            "200 OK",
            "text/html; charset=utf-8",
            INDEX_HTML.as_bytes(),
        ),

        "/api/analyze" => match request.param("word") {
            None => missing_param(&mut stream, "word"),
            Some(word) => match api::analyze(db, word) {
                Some(value) => http::respond_json(&mut stream, "200 OK", &value),
                None => http::respond_json(
                    &mut stream,
                    "404 Not Found",
                    &json!({ "error": format!("no analysis found for '{word}'") }),
                ),
            },
        },

        "/api/root" => match request.param("text") {
            None => missing_param(&mut stream, "text"),
            Some(text) => match api::root(db, text) {
                Some(value) => http::respond_json(&mut stream, "200 OK", &value),
                None => http::respond_json(
                    &mut stream,
                    "404 Not Found",
                    &json!({ "error": format!("no root morpheme found for '{text}'") }),
                ),
            },
        },

        "/api/family" => match request.param("text") {
            None => missing_param(&mut stream, "text"),
            Some(text) => http::respond_json(&mut stream, "200 OK", &api::family(db, text)),
        },

        _ => http::respond_json(
            &mut stream,
            "404 Not Found",
            &json!({ "error": "unknown endpoint" }),
        ),
    }
}

fn missing_param(stream: &mut TcpStream, name: &str) {
    http::respond_json(
        stream,
        "400 Bad Request",
        &json!({ "error": format!("missing query parameter '{name}'") }),
    );
}

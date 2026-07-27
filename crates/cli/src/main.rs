mod db;
mod format;

use std::path::PathBuf;

use anyhow::{anyhow, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lexiroot", about = "Explain English words by their morphemes.")]
struct Cli {
    /// Path to a release SQLite database.
    #[arg(long, default_value = "data/release/lexiroot-v0.1.sqlite")]
    db: PathBuf,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Decompose a word into prefix/root/suffix.
    Analyze { word: String },
    /// Look up a root morpheme by its exact form.
    Root { text: String },
    /// List known words sharing a root morpheme.
    Family { text: String },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let analyzer_db = db::load(&cli.db)?;

    match cli.command {
        Command::Analyze { word } => {
            let decomposition = analyzer_db
                .analyze(&word)
                .ok_or_else(|| anyhow!("no analysis found for '{word}'"))?;
            print!("{}", format::format_analysis(&analyzer_db, &decomposition));
        }
        Command::Root { text } => {
            let info = analyzer_db
                .root(&text)
                .ok_or_else(|| anyhow!("no root morpheme found for '{text}'"))?;
            print!("{}", format::format_root(&text, &info));
        }
        Command::Family { text } => {
            let words = analyzer_db.family(&text);
            print!("{}", format::format_family(&text, &words));
        }
    }

    Ok(())
}

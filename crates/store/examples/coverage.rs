//! Measures what fraction of a word list the engine can analyse at all.
//!
//! Usage: cargo run -p lexiroot-store --example coverage -- <wordlist> [db]

use std::path::{Path, PathBuf};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let list = args.next().expect("usage: coverage <wordlist> [db]");
    let db_path: PathBuf = args
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(lexiroot_core::RELEASE_DB_PATH).to_path_buf());

    let db = lexiroot_store::load(&db_path)?;
    let text = std::fs::read_to_string(&list)?;

    let words: Vec<&str> = text
        .lines()
        .map(|l| l.trim())
        .filter(|l| l.len() >= 4 && l.chars().all(|c| c.is_ascii_lowercase()))
        .collect();

    let analysed = words.iter().filter(|w| db.analyze(w).is_some()).count();
    println!(
        "{analysed}/{} = {:.1}%",
        words.len(),
        100.0 * analysed as f64 / words.len() as f64
    );
    Ok(())
}

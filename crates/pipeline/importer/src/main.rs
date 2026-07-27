use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lexiroot_pipeline_importer::{normalize, sqlite_writer};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("workspace root should exist relative to CARGO_MANIFEST_DIR")
}

fn main() -> Result<()> {
    let root = workspace_root();
    let colingoldberg_path = root.join("data/raw/colingoldberg-morphemes/morphemes.json");
    let withenglishwecan_path = root.join("data/raw/withenglishwecan-roots/english.roots.list.build.json");
    let output_path = root.join("data/processed.sqlite");

    let colingoldberg_json = std::fs::read_to_string(&colingoldberg_path)
        .with_context(|| format!("reading {}", colingoldberg_path.display()))?;
    let withenglishwecan_json = std::fs::read_to_string(&withenglishwecan_path)
        .with_context(|| format!("reading {}", withenglishwecan_path.display()))?;

    let (morphemes, decompositions, summary) = normalize(&colingoldberg_json, &withenglishwecan_json)?;

    sqlite_writer::write_processed_db(&output_path, &morphemes, &decompositions)
        .with_context(|| format!("writing {}", output_path.display()))?;

    println!("wrote {}", output_path.display());
    println!("  morphemes:                          {}", summary.morphemes_written);
    println!("  cross-source duplicates skipped:     {}", summary.cross_source_duplicates_skipped);
    println!("  example words seen:                  {}", summary.example_words_seen);
    println!("  precomputed decompositions:          {}", summary.precomputed_decompositions);
    println!("  skipped (no full segmentation):      {}", summary.examples_skipped_partial_segmentation);

    Ok(())
}

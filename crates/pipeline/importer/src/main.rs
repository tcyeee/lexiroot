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
    let colingoldberg_path = root.join("data/sources/colingoldberg-morphemes.json");
    let withenglishwecan_path = root.join("data/sources/withenglishwecan-roots.json");
    let lexiroot_stems_path = root.join("data/sources/lexiroot-stems.json");
    let output_path = root.join("data/build/processed.sqlite");

    let colingoldberg_json = std::fs::read_to_string(&colingoldberg_path)
        .with_context(|| format!("reading {}", colingoldberg_path.display()))?;
    let withenglishwecan_json = std::fs::read_to_string(&withenglishwecan_path)
        .with_context(|| format!("reading {}", withenglishwecan_path.display()))?;
    let lexiroot_stems_json = std::fs::read_to_string(&lexiroot_stems_path)
        .with_context(|| format!("reading {}", lexiroot_stems_path.display()))?;

    let (morphemes, decompositions, summary) =
        normalize(&colingoldberg_json, &withenglishwecan_json, &lexiroot_stems_json)?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    sqlite_writer::write_processed_db(&output_path, &morphemes, &decompositions)
        .with_context(|| format!("writing {}", output_path.display()))?;

    println!("wrote {}", output_path.display());
    println!("  morphemes:                          {}", summary.morphemes_written);
    println!("  cross-source duplicates skipped:     {}", summary.cross_source_duplicates_skipped);
    println!("  curated stems added:                 {}", summary.stems_added);
    println!("  curated stems merged into existing:  {}", summary.stems_enriched);
    println!("  morphemes carrying variants:         {}", summary.morphemes_with_variants);
    println!("  example words seen:                  {}", summary.example_words_seen);
    println!("  precomputed decompositions:          {}", summary.precomputed_decompositions);
    println!("  skipped (no full segmentation):      {}", summary.examples_skipped_partial_segmentation);

    Ok(())
}

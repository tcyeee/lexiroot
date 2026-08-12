use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lexiroot_core::{BUILD_DB_PATH, DATASET_PATH};
use lexiroot_pipeline_importer::{normalize, sqlite_writer};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("workspace root should exist relative to CARGO_MANIFEST_DIR")
}

fn main() -> Result<()> {
    let root = workspace_root();
    let dataset_path = root.join(DATASET_PATH);
    let output_path = root.join(BUILD_DB_PATH);

    let dataset_json = std::fs::read_to_string(&dataset_path)
        .with_context(|| format!("reading {}", dataset_path.display()))?;

    let (morphemes, decompositions, summary) = normalize(&dataset_json)?;

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    sqlite_writer::write_normalized_db(&output_path, &morphemes, &decompositions)
        .with_context(|| format!("writing {}", output_path.display()))?;

    println!("wrote {}", output_path.display());
    println!("  morphemes:                      {}", summary.morphemes_written);
    println!("  morphemes carrying variants:    {}", summary.morphemes_with_variants);
    println!("  example words seen:             {}", summary.example_words_seen);
    println!("  precomputed decompositions:     {}", summary.precomputed_decompositions);
    println!("  skipped (no full segmentation): {}", summary.examples_skipped_partial_segmentation);

    Ok(())
}

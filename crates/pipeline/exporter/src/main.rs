use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lexiroot_pipeline_exporter::export;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("workspace root should exist relative to CARGO_MANIFEST_DIR")
}

fn main() -> Result<()> {
    let root = workspace_root();
    let input_path = root.join("data/processed.sqlite");
    let output_dir = root.join("data/release");
    let output_path = output_dir.join("lexiroot-v0.1.sqlite");

    std::fs::create_dir_all(&output_dir)?;

    export(&input_path, &output_path)
        .with_context(|| format!("exporting {} -> {}", input_path.display(), output_path.display()))?;

    println!("wrote {}", output_path.display());

    Ok(())
}

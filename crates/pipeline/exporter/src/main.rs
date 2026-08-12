use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use lexiroot_core::{BUILD_DB_PATH, RELEASE_DB_PATH};
use lexiroot_pipeline_exporter::export;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .expect("workspace root should exist relative to CARGO_MANIFEST_DIR")
}

fn main() -> Result<()> {
    let root = workspace_root();
    let input_path = root.join(BUILD_DB_PATH);
    let output_path = root.join(RELEASE_DB_PATH);

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    export(&input_path, &output_path)
        .with_context(|| format!("exporting {} -> {}", input_path.display(), output_path.display()))?;

    println!("wrote {}", output_path.display());

    Ok(())
}

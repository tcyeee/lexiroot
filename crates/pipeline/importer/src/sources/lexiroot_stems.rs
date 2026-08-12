use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use lexiroot_core::MorphemeKind;
use serde::Deserialize;

use crate::parsed::ParsedMorpheme;

#[derive(Deserialize)]
struct RawEntry {
    positions: Vec<String>,
    #[serde(default)]
    meanings: Vec<String>,
    #[serde(default)]
    examples: Vec<String>,
    /// Irregular allomorphs only. Regular spelling changes are derived by
    /// `lexiroot_analyzer::ortho` and must not be listed — see the source
    /// README.
    #[serde(default)]
    variants: Vec<String>,
    /// Curator's note on *why* a variant is listed. Documentation only.
    #[serde(default, rename = "note")]
    _note: Option<String>,
}

/// Parses LexiRoot's own `data/sources/lexiroot-stems.json`: the curated
/// free-stem list that supplies the native Germanic core both upstream
/// Greek/Latin root dictionaries omit entirely.
///
/// Unlike the upstream sources this one is ours, so it is strict: an
/// unrecognised position is a typo in a file we control and fails the import
/// rather than being silently dropped.
pub fn parse(json: &str) -> Result<Vec<ParsedMorpheme>> {
    let raw: BTreeMap<String, RawEntry> = serde_json::from_str(json)?;
    let mut out = Vec::new();

    for (form, entry) in raw {
        if form.is_empty() {
            continue;
        }
        if entry.positions.is_empty() {
            return Err(anyhow!("lexiroot-stems entry '{form}' declares no positions"));
        }
        // One sighting per declared position; `normalize` merges them back
        // into a single morpheme carrying the full position set.
        for position in &entry.positions {
            let kind = MorphemeKind::parse(position)
                .ok_or_else(|| anyhow!("lexiroot-stems entry '{form}' has unknown position '{position}'"))?;
            out.push(ParsedMorpheme {
                form: form.clone(),
                kind,
                meanings: entry.meanings.clone(),
                evidence: format!("lexiroot-stems curated entry '{form}'"),
                examples: entry.examples.clone(),
                variants: entry.variants.clone(),
            });
        }
    }

    Ok(out)
}

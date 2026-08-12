use std::collections::BTreeMap;

use anyhow::{anyhow, Result};
use lexiroot_core::{MorphemeId, MorphemeKind, MorphemePositions};
use serde::Deserialize;

/// One morpheme as written in the curated dataset.
pub struct DatasetEntry {
    pub form: String,
    pub positions: MorphemePositions,
    pub meanings: Vec<String>,
    pub examples: Vec<String>,
    pub variants: Vec<String>,
}

/// The whole dataset, keyed by canonical morpheme id.
pub type Dataset = BTreeMap<MorphemeId, DatasetEntry>;

#[derive(Deserialize)]
struct RawEntry {
    positions: Vec<String>,
    #[serde(default)]
    meanings: Vec<String>,
    #[serde(default)]
    examples: Vec<String>,
    /// Irregular allomorphs only. Regular spelling changes are derived by
    /// `lexiroot_analyzer::ortho` and must not be listed — see the dataset
    /// README.
    #[serde(default)]
    variants: Vec<String>,
    /// Curator's note on *why* a variant is listed. Documentation only.
    #[serde(default, rename = "note")]
    _note: Option<String>,
}

/// Parses `data/sources/morphemes.json`, where one entry *is* one morpheme.
///
/// Field order is preserved exactly as written. That matters for `meanings`,
/// which is ordered most-central-first: this file used to be assembled from
/// upstream dictionaries that record one row per (form, position) *sighting*,
/// and folding those sightings together deduplicated — and so alphabetized —
/// the meanings. There is nothing to fold here, so the curator's ordering is
/// the ordering that ships.
///
/// Strict on purpose: an unrecognised position is a typo in a file we own and
/// fails the import rather than being silently dropped.
pub fn parse(json: &str) -> Result<Dataset> {
    let raw: BTreeMap<String, RawEntry> = serde_json::from_str(json)?;
    let mut dataset = Dataset::new();

    for (form, entry) in raw {
        if form.is_empty() {
            continue;
        }
        if entry.positions.is_empty() {
            return Err(anyhow!("entry '{form}' declares no positions"));
        }

        let mut positions = MorphemePositions::empty();
        for position in &entry.positions {
            let kind = MorphemeKind::parse(position)
                .ok_or_else(|| anyhow!("entry '{form}' has unknown position '{position}'"))?;
            positions.insert(kind);
        }

        let id = MorphemeId::new(&form);
        if dataset.contains_key(&id) {
            // Two keys differing only in case collide on the lowercased id;
            // without this one would silently overwrite the other.
            return Err(anyhow!(
                "two entries share the id '{}' (form '{form}')",
                id.as_str()
            ));
        }
        dataset.insert(
            id,
            DatasetEntry {
                form,
                positions,
                meanings: entry.meanings,
                examples: entry.examples,
                variants: entry.variants,
            },
        );
    }

    Ok(dataset)
}

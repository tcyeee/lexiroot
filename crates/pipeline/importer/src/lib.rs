pub mod dataset;
pub mod sqlite_writer;

use std::collections::BTreeMap;

use anyhow::Result;
use lexiroot_analyzer::{segment_ranked, MorphemeIndex};
use lexiroot_core::{Morpheme, MorphemeRef, Provenance, WordDecomposition};

pub use dataset::{Dataset, DatasetEntry};

/// Confidence carried by every curated record. The analyzer's query-time
/// fallback uses a lower one (see `AnalyzerDb`), which is what still tells a
/// dataset entry apart from a derived one now that records carry no source id.
const CURATED_CONFIDENCE: f32 = 0.95;

#[derive(Debug, Default, Clone, Copy)]
pub struct ImportSummary {
    pub morphemes_written: usize,
    pub morphemes_with_variants: usize,
    pub example_words_seen: usize,
    pub precomputed_decompositions: usize,
    pub examples_skipped_partial_segmentation: usize,
}

/// Reads the curated dataset and builds the two record tables: every morpheme,
/// plus a precomputed `WordDecomposition` for each example word that segments
/// with full coverage against them (see the design plan's rationale for
/// reusing `lexiroot_analyzer::segment` here rather than duplicating the
/// algorithm).
pub fn normalize(
    dataset_json: &str,
) -> Result<(Vec<Morpheme>, Vec<WordDecomposition>, ImportSummary)> {
    build_tables(dataset::parse(dataset_json)?)
}

/// Everything downstream of parsing, exposed separately so a caller holding a
/// `Dataset` in memory can build the tables without re-serializing it.
pub fn build_tables(
    dataset: Dataset,
) -> Result<(Vec<Morpheme>, Vec<WordDecomposition>, ImportSummary)> {
    let mut examples_by_word: BTreeMap<String, String> = BTreeMap::new();
    for entry in dataset.values() {
        for example in &entry.examples {
            examples_by_word
                .entry(example.to_lowercase())
                .or_insert_with(|| example.clone());
        }
    }

    let mut morphemes = Vec::with_capacity(dataset.len());
    for (_, entry) in dataset {
        let evidence = format!("curated dataset entry '{}'", entry.form);
        let provenance = Provenance::new(CURATED_CONFIDENCE, evidence)?;
        morphemes.push(Morpheme::with_variants(
            entry.form,
            entry.positions,
            entry.meanings,
            entry.variants,
            provenance,
        ));
    }
    morphemes.sort_unstable_by(|a, b| a.id.cmp(&b.id));

    let index = MorphemeIndex::build(morphemes.clone());
    let mut precomputed = Vec::new();
    let mut skipped_partial = 0usize;
    for original_word in examples_by_word.values() {
        match segment_ranked(original_word, &index, 1).into_iter().next() {
            Some(parse) => {
                let forms: Vec<&str> = parse
                    .segments
                    .iter()
                    .filter_map(|s: &MorphemeRef| index.get(&s.morpheme_id))
                    .map(|m| m.form.as_str())
                    .collect();
                // Same reasoning as the analyzer's tier-2 evidence: the forms
                // are canonical, so a row for `believable` cites the root
                // `believe` and has to say why.
                let spelling = match parse.root_rule {
                    Some(rule) => format!(", after reversing {} at the root boundary", rule.as_str()),
                    None => String::new(),
                };
                let evidence = format!(
                    "dataset example word, segmented into: {}{spelling}",
                    forms.join(" + ")
                );
                let provenance = Provenance::new(CURATED_CONFIDENCE, evidence)?;
                precomputed.push(WordDecomposition {
                    word: original_word.clone(),
                    segments: parse.segments,
                    provenance,
                });
            }
            None => skipped_partial += 1,
        }
    }
    precomputed.sort_unstable_by(|a, b| a.word.cmp(&b.word));

    let summary = ImportSummary {
        morphemes_written: morphemes.len(),
        morphemes_with_variants: morphemes.iter().filter(|m| !m.variants.is_empty()).count(),
        example_words_seen: examples_by_word.len(),
        precomputed_decompositions: precomputed.len(),
        examples_skipped_partial_segmentation: skipped_partial,
    };

    Ok((morphemes, precomputed, summary))
}

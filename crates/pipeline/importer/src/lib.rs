mod parsed;
pub mod sources;
pub mod sqlite_writer;

use std::collections::BTreeMap;

use anyhow::Result;
use lexiroot_analyzer::{segment_ranked, MorphemeIndex};
use lexiroot_core::{
    Morpheme, MorphemeId, MorphemePositions, MorphemeRef, Provenance, SourceId, WordDecomposition,
};

use parsed::ParsedMorpheme;

#[derive(Debug, Default, Clone, Copy)]
pub struct ImportSummary {
    pub morphemes_written: usize,
    pub cross_source_duplicates_skipped: usize,
    /// Curated stems that were new to the table — the Germanic core the two
    /// Greek/Latin dictionaries never had.
    pub stems_added: usize,
    /// Curated stems that merged into an existing entry, adding a position,
    /// examples or variants to it.
    pub stems_enriched: usize,
    pub morphemes_with_variants: usize,
    pub example_words_seen: usize,
    pub precomputed_decompositions: usize,
    pub examples_skipped_partial_segmentation: usize,
}

fn dedup_sorted(mut items: Vec<String>) -> Vec<String> {
    items.sort_unstable();
    items.dedup();
    items
}

/// All sightings of one form, collapsed. `positions` accumulates every slot
/// any source attested the form in.
struct Merged {
    form: String,
    positions: MorphemePositions,
    meanings: Vec<String>,
    evidence: String,
    examples: Vec<String>,
    variants: Vec<String>,
    source: SourceId,
}

impl Merged {
    fn new(pm: ParsedMorpheme, source: SourceId) -> Self {
        Self {
            form: pm.form,
            positions: MorphemePositions::from_kind(pm.kind),
            meanings: pm.meanings,
            evidence: pm.evidence,
            examples: pm.examples,
            variants: dedup_sorted(pm.variants),
            source,
        }
    }
}

fn merge_into(existing: &mut Merged, incoming: &ParsedMorpheme) {
    existing.positions.insert(incoming.kind);
    existing.meanings.extend(incoming.meanings.iter().cloned());
    existing.meanings = dedup_sorted(std::mem::take(&mut existing.meanings));
    existing.examples.extend(incoming.examples.iter().cloned());
    existing.examples = dedup_sorted(std::mem::take(&mut existing.examples));
    existing.variants.extend(incoming.variants.iter().cloned());
    existing.variants = dedup_sorted(std::mem::take(&mut existing.variants));
    existing.evidence.push_str("; ");
    existing.evidence.push_str(&incoming.evidence);
}

/// Merges colingoldberg (primary), withenglishwecan (secondary) and
/// lexiroot-stems records into a deduplicated `Morpheme` table, then
/// precomputes `WordDecomposition` rows for every example word that segments
/// with full coverage against that table (see the design plan's rationale for
/// reusing `lexiroot_analyzer::segment` here rather than duplicating the
/// algorithm).
///
/// The stem list is applied last and, unlike the two upstream dictionaries,
/// is allowed to *add to* an existing entry rather than being skipped as a
/// duplicate: its whole job is to supply what those dictionaries lack. A form
/// they list only as an affix (`work` is suffix-only upstream) gets its root
/// position here, and its `variants` are the only source of allomorphs.
pub fn normalize(
    colingoldberg_json: &str,
    withenglishwecan_json: &str,
    lexiroot_stems_json: &str,
) -> Result<(Vec<Morpheme>, Vec<WordDecomposition>, ImportSummary)> {
    let primary = sources::colingoldberg::parse(colingoldberg_json)?;
    let secondary = sources::withenglishwecan::parse(withenglishwecan_json)?;
    let stems = sources::lexiroot_stems::parse(lexiroot_stems_json)?;

    let mut by_id: BTreeMap<MorphemeId, Merged> = BTreeMap::new();
    let mut duplicates_skipped = 0usize;

    // Keyed on form alone, so a form the source lists under several `loc`
    // values becomes one morpheme carrying all of them rather than two or
    // three morphemes each missing the others' positions.
    for pm in primary {
        let id = MorphemeId::new(&pm.form);
        match by_id.get_mut(&id) {
            Some(existing) => merge_into(existing, &pm),
            None => {
                by_id.insert(id, Merged::new(pm, SourceId::ColinGoldbergMorphemes));
            }
        }
    }
    for pm in secondary {
        let id = MorphemeId::new(&pm.form);
        if let Some(existing) = by_id.get_mut(&id) {
            // The primary source stays authoritative for meanings and
            // provenance, but a root sighting here is still evidence the
            // form can head a word, so the position is worth keeping.
            existing.positions.insert(pm.kind);
            duplicates_skipped += 1;
            continue;
        }
        by_id.insert(id, Merged::new(pm, SourceId::WithEnglishWeCanRoots));
    }

    let mut stems_added = 0usize;
    let mut stems_enriched = 0usize;
    for pm in stems {
        let id = MorphemeId::new(&pm.form);
        match by_id.get_mut(&id) {
            // Full merge, not a skip: an upstream entry for the same spelling
            // is usually a bound Greek/Latin root that happens to collide
            // (`mark` the boundary-marker root vs. the free stem), and the
            // curated entry's positions, examples and variants all still
            // apply to the merged morpheme.
            Some(existing) => {
                merge_into(existing, &pm);
                stems_enriched += 1;
            }
            None => {
                by_id.insert(id, Merged::new(pm, SourceId::LexirootStems));
                stems_added += 1;
            }
        }
    }

    let mut examples_by_word: BTreeMap<String, (String, SourceId)> = BTreeMap::new();
    for merged in by_id.values() {
        for example in &merged.examples {
            let key = example.to_lowercase();
            examples_by_word
                .entry(key)
                .or_insert_with(|| (example.clone(), merged.source));
        }
    }

    let mut morphemes = Vec::with_capacity(by_id.len());
    for (_, merged) in by_id {
        let provenance = Provenance::new(merged.source, 0.95, merged.evidence)?;
        morphemes.push(Morpheme::with_variants(
            merged.form,
            merged.positions,
            merged.meanings,
            merged.variants,
            provenance,
        ));
    }
    morphemes.sort_unstable_by(|a, b| a.id.cmp(&b.id));

    let index = MorphemeIndex::build(morphemes.clone());
    let mut precomputed = Vec::new();
    let mut skipped_partial = 0usize;
    for (original_word, source) in examples_by_word.values() {
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
                    "source-listed example word, segmented into: {}{spelling}",
                    forms.join(" + ")
                );
                let provenance = Provenance::new(*source, 0.95, evidence)?;
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
        cross_source_duplicates_skipped: duplicates_skipped,
        stems_added,
        stems_enriched,
        morphemes_with_variants: morphemes.iter().filter(|m| !m.variants.is_empty()).count(),
        example_words_seen: examples_by_word.len(),
        precomputed_decompositions: precomputed.len(),
        examples_skipped_partial_segmentation: skipped_partial,
    };

    Ok((morphemes, precomputed, summary))
}

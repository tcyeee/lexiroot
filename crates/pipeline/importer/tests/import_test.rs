use lexiroot_core::{MorphemeId, MorphemeKind, SourceId};
use lexiroot_pipeline_importer::normalize;

const COLINGOLDBERG_FIXTURE: &str = include_str!("fixtures/colingoldberg.json");
const WITHENGLISHWECAN_FIXTURE: &str = include_str!("fixtures/withenglishwecan.json");
const LEXIROOT_STEMS_FIXTURE: &str = include_str!("fixtures/lexiroot_stems.json");

fn normalized() -> (
    Vec<lexiroot_core::Morpheme>,
    Vec<lexiroot_core::WordDecomposition>,
    lexiroot_pipeline_importer::ImportSummary,
) {
    normalize(COLINGOLDBERG_FIXTURE, WITHENGLISHWECAN_FIXTURE, LEXIROOT_STEMS_FIXTURE).unwrap()
}

#[test]
fn normalize_merges_dedupes_and_precomputes_decompositions() {
    let (morphemes, decompositions, summary) = normalized();

    // 8 distinct forms: in, spect, vis, able, hand, duc from the two
    // dictionaries, plus believe and admit from the curated stem list.
    // "vis" from the secondary source collides with the primary and is
    // skipped, not counted as a 9th; "hand" from the stem list merges into
    // the existing entry rather than being added.
    assert_eq!(summary.morphemes_written, 8);
    assert_eq!(morphemes.len(), 8);
    assert_eq!(summary.cross_source_duplicates_skipped, 1);
    assert_eq!(summary.stems_added, 2);
    assert_eq!(summary.stems_enriched, 1);

    // Within-source duplicate "able" entries (able-a, able-b) merge their
    // meanings rather than overwriting.
    let able = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("able"))
        .expect("able should exist");
    assert_eq!(able.meanings, vec!["capable of".to_string(), "worthy".to_string()]);

    // The Tier 0 fix: one entry listing "hand" under two `loc` values becomes
    // a single morpheme carrying both positions, not two morphemes each
    // missing the other's. The curated stem list then adds the root position
    // the dictionaries never recorded — that enrichment is the whole point of
    // applying it as a merge rather than skipping it as a duplicate.
    let hand = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("hand"))
        .expect("hand should exist");
    assert!(hand.positions.contains(MorphemeKind::Prefix));
    assert!(hand.positions.contains(MorphemeKind::Suffix));
    assert!(hand.positions.contains(MorphemeKind::Root));
    assert_eq!(hand.positions.to_storage_string(), "prefix|root|suffix");

    // Cross-source duplicate "vis" keeps the primary source's meaning, not
    // the secondary's.
    let vis = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("vis"))
        .expect("vis should exist");
    assert_eq!(vis.meanings, vec!["see".to_string()]);
    assert_eq!(vis.provenance.source, SourceId::ColinGoldbergMorphemes);

    // "duc" only exists in the secondary source and should still be added
    // since there's no collision.
    assert!(morphemes.iter().any(|m| m.id == MorphemeId::new("duc")));

    // "believable", "duc", "induc", "inspect", "invis" all fully segment
    // against the merged morpheme table and get precomputed; "spectrogram"
    // doesn't (no suffix "-rogram"/"-gram" is known) and is skipped rather
    // than recorded with a wrong decomposition.
    assert_eq!(summary.example_words_seen, 6);
    assert_eq!(summary.precomputed_decompositions, 5);
    assert_eq!(summary.examples_skipped_partial_segmentation, 1);

    let words: Vec<&str> = decompositions.iter().map(|d| d.word.as_str()).collect();
    assert_eq!(words, vec!["believable", "duc", "induc", "inspect", "invis"]);
    assert!(!words.contains(&"spectrogram"));

    let inspect = decompositions.iter().find(|d| d.word == "inspect").unwrap();
    assert_eq!(inspect.segments.len(), 2);
    assert_eq!(inspect.segments[0].morpheme_id, MorphemeId::new("in"));
    assert_eq!(inspect.segments[0].role, MorphemeKind::Prefix);
    assert_eq!(inspect.segments[1].morpheme_id, MorphemeId::new("spect"));
    assert_eq!(inspect.segments[1].role, MorphemeKind::Root);
    assert_eq!(inspect.provenance.source, SourceId::ColinGoldbergMorphemes);

    let duc = decompositions.iter().find(|d| d.word == "duc").unwrap();
    assert_eq!(duc.segments.len(), 1);
    assert_eq!(duc.segments[0].role, MorphemeKind::Root);
    assert_eq!(duc.provenance.source, SourceId::WithEnglishWeCanRoots);
}

/// Irregular allomorphs survive the merge onto the `Morpheme` the importer
/// emits — the field the release schema's `variants` column is written from.
#[test]
fn curated_variants_reach_the_emitted_morpheme() {
    let (morphemes, _, summary) = normalized();

    let admit = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("admit"))
        .expect("admit should exist");
    assert_eq!(admit.variants, vec!["admiss".to_string()]);
    assert_eq!(admit.provenance.source, SourceId::LexirootStems);

    // Everything else carries none — variants are for irregular alternation
    // only, so this stays near-empty as the table grows.
    assert_eq!(summary.morphemes_with_variants, 1);
    assert!(morphemes
        .iter()
        .filter(|m| m.id != MorphemeId::new("admit"))
        .all(|m| m.variants.is_empty()));
}

/// The pipeline path for the regular rules: `believable` is spelled with
/// `believ`, the table holds `believe`, and the precomputed row has to come
/// out under the canonical id or it would reference a morpheme that does not
/// exist.
#[test]
fn precomputed_decomposition_uses_the_canonical_id_after_a_spelling_rule() {
    let (_, decompositions, _) = normalized();

    let believable = decompositions
        .iter()
        .find(|d| d.word == "believable")
        .expect("believable should be precomputed");
    let segments: Vec<(&str, MorphemeKind)> = believable
        .segments
        .iter()
        .map(|s| (s.morpheme_id.as_str(), s.role))
        .collect();
    assert_eq!(
        segments,
        vec![("believe", MorphemeKind::Root), ("able", MorphemeKind::Suffix)]
    );
}

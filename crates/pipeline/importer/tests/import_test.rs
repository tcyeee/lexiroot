use lexiroot_core::{MorphemeId, MorphemeKind, SourceId};
use lexiroot_pipeline_importer::normalize;

const COLINGOLDBERG_FIXTURE: &str = include_str!("fixtures/colingoldberg.json");
const WITHENGLISHWECAN_FIXTURE: &str = include_str!("fixtures/withenglishwecan.json");

#[test]
fn normalize_merges_dedupes_and_precomputes_decompositions() {
    let (morphemes, decompositions, summary) = normalize(COLINGOLDBERG_FIXTURE, WITHENGLISHWECAN_FIXTURE).unwrap();

    // 5 distinct (kind, form) morphemes: prefix:in, root:spect, root:vis,
    // root:duc, suffix:able. "root:vis" from the secondary source collides
    // with the primary and is skipped, not counted as a 6th.
    assert_eq!(summary.morphemes_written, 5);
    assert_eq!(morphemes.len(), 5);
    assert_eq!(summary.cross_source_duplicates_skipped, 1);

    // Within-source duplicate suffix:able entries (able-a, able-b) merge
    // their meanings rather than overwriting.
    let able = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new(MorphemeKind::Suffix, "able"))
        .expect("suffix:able should exist");
    assert_eq!(able.meanings, vec!["capable of".to_string(), "worthy".to_string()]);

    // Cross-source duplicate root:vis keeps the primary source's meaning,
    // not the secondary's.
    let vis = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new(MorphemeKind::Root, "vis"))
        .expect("root:vis should exist");
    assert_eq!(vis.meanings, vec!["see".to_string()]);
    assert_eq!(vis.provenance.source, SourceId::ColinGoldbergMorphemes);

    // root:duc only exists in the secondary source and should still be
    // added since there's no collision.
    assert!(morphemes.iter().any(|m| m.id == MorphemeId::new(MorphemeKind::Root, "duc")));

    // "duc", "induc", "inspect", "invis" all fully segment against the
    // merged morpheme table and get precomputed; "spectrogram" doesn't (no
    // suffix "-rogram"/"-gram" is known) and is skipped rather than
    // recorded with a wrong decomposition.
    assert_eq!(summary.example_words_seen, 5);
    assert_eq!(summary.precomputed_decompositions, 4);
    assert_eq!(summary.examples_skipped_partial_segmentation, 1);

    let words: Vec<&str> = decompositions.iter().map(|d| d.word.as_str()).collect();
    assert_eq!(words, vec!["duc", "induc", "inspect", "invis"]);
    assert!(!words.contains(&"spectrogram"));

    let inspect = decompositions.iter().find(|d| d.word == "inspect").unwrap();
    assert_eq!(inspect.segments.len(), 2);
    assert_eq!(inspect.segments[0].morpheme_id, MorphemeId::new(MorphemeKind::Prefix, "in"));
    assert_eq!(inspect.segments[1].morpheme_id, MorphemeId::new(MorphemeKind::Root, "spect"));
    assert_eq!(inspect.provenance.source, SourceId::ColinGoldbergMorphemes);

    let duc = decompositions.iter().find(|d| d.word == "duc").unwrap();
    assert_eq!(duc.segments.len(), 1);
    assert_eq!(duc.provenance.source, SourceId::WithEnglishWeCanRoots);
}

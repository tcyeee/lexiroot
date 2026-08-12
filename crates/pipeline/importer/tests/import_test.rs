use lexiroot_core::{MorphemeId, MorphemeKind};
use lexiroot_pipeline_importer::normalize;

/// A miniature of the real dataset. `admit`'s meanings are deliberately not in
/// alphabetical order — see `meanings_keep_the_curators_order`.
const DATASET: &str = r#"{
  "able":    { "positions": ["suffix"], "meanings": ["capable of", "worthy"], "examples": ["believable"] },
  "admit":   { "positions": ["root"], "meanings": ["let in", "confess"], "variants": ["admiss"] },
  "believe": { "positions": ["root"], "meanings": ["accept as true", "have faith in"], "examples": ["believable"] },
  "hand":    { "positions": ["prefix", "root", "suffix"], "meanings": ["hand"] },
  "in":      { "positions": ["prefix"], "meanings": ["into", "not"], "examples": ["inspect"] },
  "spect":   { "positions": ["root"], "meanings": ["look", "see"], "examples": ["inspect", "spectrogram"] }
}"#;

fn normalized() -> (
    Vec<lexiroot_core::Morpheme>,
    Vec<lexiroot_core::WordDecomposition>,
    lexiroot_pipeline_importer::ImportSummary,
) {
    normalize(DATASET).unwrap()
}

#[test]
fn normalize_emits_every_entry_and_precomputes_decompositions() {
    let (morphemes, decompositions, summary) = normalized();

    assert_eq!(summary.morphemes_written, 6);
    assert_eq!(morphemes.len(), 6);

    // One entry declaring three positions stays one morpheme carrying all
    // three, not three morphemes each missing the others'. That collapse is
    // what lets the segmenter decide, per word, which slot a form fills.
    let hand = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("hand"))
        .expect("hand should exist");
    assert_eq!(hand.positions.to_storage_string(), "prefix|root|suffix");

    // `believable`, `inspect` and `spectrogram` are all listed as examples;
    // the first two segment with full coverage, `spectrogram` does not (no
    // `-rogram` or `-gram` suffix is known) and is skipped rather than
    // recorded with a wrong decomposition.
    assert_eq!(summary.example_words_seen, 3);
    assert_eq!(summary.precomputed_decompositions, 2);
    assert_eq!(summary.examples_skipped_partial_segmentation, 1);

    let words: Vec<&str> = decompositions.iter().map(|d| d.word.as_str()).collect();
    assert_eq!(words, vec!["believable", "inspect"]);

    let inspect = decompositions.iter().find(|d| d.word == "inspect").unwrap();
    assert_eq!(inspect.segments.len(), 2);
    assert_eq!(inspect.segments[0].morpheme_id, MorphemeId::new("in"));
    assert_eq!(inspect.segments[0].role, MorphemeKind::Prefix);
    assert_eq!(inspect.segments[1].morpheme_id, MorphemeId::new("spect"));
    assert_eq!(inspect.segments[1].role, MorphemeKind::Root);
}

/// Meanings are ordered most-central-first by the curator, so the importer
/// must not reorder them. It used to: the dataset was assembled from upstream
/// files recording one row per (form, position) sighting, and folding those
/// sightings deduplicated — and so alphabetized — the meanings. Sorting
/// `admit` here would silently promote "confess" over "let in".
#[test]
fn meanings_keep_the_curators_order() {
    let (morphemes, _, _) = normalized();

    let admit = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("admit"))
        .expect("admit should exist");
    assert_eq!(admit.meanings, vec!["let in".to_string(), "confess".to_string()]);
}

/// Irregular allomorphs survive onto the `Morpheme` the importer emits — the
/// field the release schema's `variants` column is written from.
#[test]
fn variants_reach_the_emitted_morpheme() {
    let (morphemes, _, summary) = normalized();

    let admit = morphemes
        .iter()
        .find(|m| m.id == MorphemeId::new("admit"))
        .expect("admit should exist");
    assert_eq!(admit.variants, vec!["admiss".to_string()]);

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

/// The dataset is ours, so a malformed entry is a typo worth failing on rather
/// than dropping silently.
#[test]
fn malformed_entries_fail_the_import() {
    let unknown_position = r#"{ "spect": { "positions": ["stem"], "meanings": ["look"] } }"#;
    assert!(normalize(unknown_position).is_err());

    let no_positions = r#"{ "spect": { "positions": [], "meanings": ["look"] } }"#;
    assert!(normalize(no_positions).is_err());

    // Ids are lowercased forms, so two keys differing only in case would
    // collide and one would silently win.
    let case_collision =
        r#"{ "Spect": { "positions": ["root"], "meanings": ["look"] }, "spect": { "positions": ["root"], "meanings": ["see"] } }"#;
    assert!(normalize(case_collision).is_err());
}

use lexiroot_analyzer::AnalyzerDb;
use lexiroot_core::{Morpheme, MorphemeId, MorphemeKind, MorphemeRef, Provenance, SourceId, WordDecomposition};

fn morpheme(form: &str, kind: MorphemeKind, meaning: &str) -> Morpheme {
    Morpheme::new(
        form,
        kind,
        vec![meaning.to_string()],
        Provenance::new(SourceId::ColinGoldbergMorphemes, 0.95, format!("fixture entry '{form}'")).unwrap(),
    )
}

fn fixture_db() -> AnalyzerDb {
    let morphemes = vec![
        morpheme("un", MorphemeKind::Prefix, "not"),
        morpheme("believe", MorphemeKind::Root, "have faith"),
        morpheme("break", MorphemeKind::Root, "separate into pieces"),
        morpheme("able", MorphemeKind::Suffix, "capable of"),
    ];

    let precomputed = vec![WordDecomposition {
        word: "unbelievable".to_string(),
        segments: vec![
            MorphemeRef {
                morpheme_id: MorphemeId::new(MorphemeKind::Prefix, "un"),
            },
            MorphemeRef {
                morpheme_id: MorphemeId::new(MorphemeKind::Root, "believe"),
            },
            MorphemeRef {
                morpheme_id: MorphemeId::new(MorphemeKind::Suffix, "able"),
            },
        ],
        provenance: Provenance::new(
            SourceId::ColinGoldbergMorphemes,
            0.95,
            "example word for 'believe' per fixture",
        )
        .unwrap(),
    }];

    AnalyzerDb::new(morphemes, precomputed)
}

#[test]
fn tier1_returns_precomputed_source_listed_decomposition() {
    let db = fixture_db();
    let result = db.analyze("unbelievable").expect("should hit precomputed table");
    assert_eq!(result.segments.len(), 3);
    assert_eq!(result.provenance.source, SourceId::ColinGoldbergMorphemes);
    assert_eq!(result.provenance.confidence(), 0.95);
}

#[test]
fn tier1_lookup_is_case_insensitive() {
    let db = fixture_db();
    let result = db.analyze("UnBelievable").expect("should hit precomputed table regardless of case");
    assert_eq!(result.segments.len(), 3);
}

#[test]
fn tier2_falls_back_to_live_algorithmic_segmentation() {
    let db = fixture_db();
    // "unbreakable" is not in the precomputed table but is composed of
    // known affixes (un- + break + -able) with no spelling changes at the
    // boundaries, so tier 2 should segment it live via plain substring
    // matching.
    let result = db.analyze("unbreakable").expect("should segment via algorithmic fallback");
    assert_eq!(result.segments.len(), 3);
    assert_eq!(result.provenance.source, SourceId::Inferred);
    assert_eq!(result.provenance.confidence(), 0.5);
    assert!(result.provenance.evidence.contains("un"));
    assert!(result.provenance.evidence.contains("break"));
    assert!(result.provenance.evidence.contains("able"));
}

#[test]
fn tier3_returns_none_when_nothing_matches() {
    let db = fixture_db();
    assert!(db.analyze("zzznotaword").is_none());
}

#[test]
fn root_returns_meanings_and_related_words() {
    let db = fixture_db();
    let info = db.root("believe").expect("believe is a known root");
    assert_eq!(info.morpheme.meanings, vec!["have faith".to_string()]);
    assert_eq!(info.related_words, vec!["unbelievable"]);
}

#[test]
fn root_returns_none_for_unknown_form() {
    let db = fixture_db();
    assert!(db.root("zzznotaroot").is_none());
}

#[test]
fn family_groups_precomputed_words_by_shared_root() {
    let db = fixture_db();
    let words: Vec<&str> = db.family("believe").iter().map(|d| d.word.as_str()).collect();
    assert_eq!(words, vec!["unbelievable"]);
}

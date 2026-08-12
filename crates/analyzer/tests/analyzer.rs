use lexiroot_analyzer::{segment, AnalyzerDb, MorphemeIndex};
use lexiroot_core::{
    Morpheme, MorphemeId, MorphemeKind, MorphemePositions, MorphemeRef, Provenance, SourceId,
    WordDecomposition,
};

fn positions(kinds: &[MorphemeKind]) -> MorphemePositions {
    let mut p = MorphemePositions::empty();
    for k in kinds {
        p.insert(*k);
    }
    p
}

fn morpheme(form: &str, kinds: &[MorphemeKind], meaning: &str) -> Morpheme {
    Morpheme::new(
        form,
        positions(kinds),
        vec![meaning.to_string()],
        Provenance::new(SourceId::ColinGoldbergMorphemes, 0.95, format!("fixture entry '{form}'")).unwrap(),
    )
}

fn seg(form: &str, role: MorphemeKind) -> MorphemeRef {
    MorphemeRef {
        morpheme_id: MorphemeId::new(form),
        role,
    }
}

fn fixture_morphemes() -> Vec<Morpheme> {
    use MorphemeKind::*;
    vec![
        morpheme("un", &[Prefix], "not"),
        morpheme("believe", &[Root], "have faith"),
        morpheme("break", &[Root], "separate into pieces"),
        morpheme("able", &[Suffix], "capable of"),
        // Attested in two positions at once — the case that used to produce
        // two separate morphemes, neither usable as the other's position.
        morpheme("trans", &[Prefix, Root], "across"),
        morpheme("port", &[Prefix, Root, Suffix], "carry"),
        morpheme("ation", &[Suffix], "act of"),
        // Long enough to stand in as a root, but only ever seen as a suffix.
        morpheme("scope", &[Suffix], "look at"),
        morpheme("micro", &[Prefix], "small"),
        // A one-character affix, of the kind that dominates the real data.
        morpheme("e", &[Prefix, Suffix], "noise"),
    ]
}

fn fixture_index() -> MorphemeIndex {
    MorphemeIndex::build(fixture_morphemes())
}

fn fixture_db() -> AnalyzerDb {
    let precomputed = vec![WordDecomposition {
        word: "unbelievable".to_string(),
        segments: vec![
            seg("un", MorphemeKind::Prefix),
            seg("believe", MorphemeKind::Root),
            seg("able", MorphemeKind::Suffix),
        ],
        provenance: Provenance::new(
            SourceId::ColinGoldbergMorphemes,
            0.95,
            "example word for 'believe' per fixture",
        )
        .unwrap(),
    }];

    AnalyzerDb::new(fixture_morphemes(), precomputed)
}

fn forms(word: &str, index: &MorphemeIndex) -> Option<Vec<(String, MorphemeKind)>> {
    segment(word, index).map(|segments| {
        segments
            .iter()
            .map(|s| (s.morpheme_id.as_str().to_string(), s.role))
            .collect()
    })
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
    // boundaries, so tier 2 should segment it live.
    let result = db.analyze("unbreakable").expect("should segment via algorithmic fallback");
    assert_eq!(result.segments.len(), 3);
    assert_eq!(result.provenance.source, SourceId::Inferred);
    assert_eq!(result.provenance.confidence(), 0.5);
    assert!(result.provenance.evidence.contains("break"));
}

#[test]
fn tier3_returns_none_when_nothing_matches() {
    let db = fixture_db();
    assert!(db.analyze("zzznotaword").is_none());
}

/// The regression that motivated the position-set rewrite: with one morpheme
/// per (form, position), `trans` and `port` were separate morphemes and the
/// greedy pass could only ever strip one prefix and one suffix.
#[test]
fn prefers_prefix_plus_root_over_root_plus_suffix() {
    let index = fixture_index();
    assert_eq!(
        forms("transport", &index),
        Some(vec![
            ("trans".into(), MorphemeKind::Prefix),
            ("port".into(), MorphemeKind::Root),
        ])
    );
}

#[test]
fn segments_prefix_root_and_suffix_together() {
    let index = fixture_index();
    assert_eq!(
        forms("transportation", &index),
        Some(vec![
            ("trans".into(), MorphemeKind::Prefix),
            ("port".into(), MorphemeKind::Root),
            ("ation".into(), MorphemeKind::Suffix),
        ])
    );
}

/// `scope` is only attested as a suffix, so it fills the root slot under
/// penalty rather than being rejected outright.
#[test]
fn long_affix_only_form_can_stand_in_as_a_root() {
    let index = fixture_index();
    assert_eq!(
        forms("microscope", &index),
        Some(vec![
            ("micro".into(), MorphemeKind::Prefix),
            ("scope".into(), MorphemeKind::Root),
        ])
    );
}

/// One-character affixes are excluded: admitting them lets the search shred
/// any word into letters.
#[test]
fn one_character_affixes_are_never_used() {
    let index = fixture_index();
    let parsed = forms("ebreake", &index);
    assert!(
        parsed.is_none(),
        "should not segment via the one-character affix 'e', got {parsed:?}"
    );
}

#[test]
fn exact_root_match_returns_a_single_segment() {
    let index = fixture_index();
    assert_eq!(forms("break", &index), Some(vec![("break".into(), MorphemeKind::Root)]));
}

#[test]
fn root_returns_meanings_and_related_words() {
    let db = fixture_db();
    let info = db.root("believe").expect("believe is a known root");
    assert_eq!(info.morpheme.meanings, vec!["have faith".to_string()]);
    assert_eq!(info.related_words, vec!["unbelievable"]);
}

/// `root` reports only forms a source actually called a root, so the
/// segmenter's affix-only stand-in must not leak into this query path.
#[test]
fn root_returns_none_for_affix_only_form() {
    let db = fixture_db();
    assert!(db.root("scope").is_none());
    assert!(db.root("zzznotaroot").is_none());
}

#[test]
fn family_groups_precomputed_words_by_shared_root() {
    let db = fixture_db();
    let words: Vec<&str> = db.family("believe").iter().map(|d| d.word.as_str()).collect();
    assert_eq!(words, vec!["unbelievable"]);
}

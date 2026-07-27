use lexiroot_core::{Morpheme, MorphemeId, MorphemeKind, Provenance, SourceId, WordDecomposition, MorphemeRef};

fn source_listed(evidence: &str) -> Provenance {
    Provenance::new(SourceId::ColinGoldbergMorphemes, 0.95, evidence).unwrap()
}

#[test]
fn provenance_rejects_confidence_out_of_range() {
    assert!(Provenance::new(SourceId::Inferred, -0.01, "x").is_err());
    assert!(Provenance::new(SourceId::Inferred, 1.01, "x").is_err());
}

#[test]
fn provenance_accepts_boundary_confidence() {
    assert!(Provenance::new(SourceId::Inferred, 0.0, "x").is_ok());
    assert!(Provenance::new(SourceId::Inferred, 1.0, "x").is_ok());
}

#[test]
fn morpheme_id_is_deterministic_and_lowercased() {
    let a = MorphemeId::new(MorphemeKind::Root, "Spect");
    let b = MorphemeId::new(MorphemeKind::Root, "spect");
    assert_eq!(a, b);
    assert_eq!(a.as_str(), "root:spect");
}

#[test]
fn morpheme_serde_round_trip() {
    let m = Morpheme::new(
        "spect",
        MorphemeKind::Root,
        vec!["look".into(), "see".into()],
        source_listed("colingoldberg/morphemes entry 'spect'"),
    );
    let json = serde_json::to_string(&m).unwrap();
    let back: Morpheme = serde_json::from_str(&json).unwrap();
    assert_eq!(m, back);
}

#[test]
fn word_decomposition_serde_round_trip() {
    let root_id = MorphemeId::new(MorphemeKind::Root, "spect");
    let decomposition = WordDecomposition {
        word: "inspect".into(),
        segments: vec![
            MorphemeRef {
                morpheme_id: MorphemeId::new(MorphemeKind::Prefix, "in"),
            },
            MorphemeRef {
                morpheme_id: root_id,
            },
        ],
        provenance: source_listed("example word for 'spect' per colingoldberg/morphemes"),
    };
    let json = serde_json::to_string(&decomposition).unwrap();
    let back: WordDecomposition = serde_json::from_str(&json).unwrap();
    assert_eq!(decomposition, back);
}

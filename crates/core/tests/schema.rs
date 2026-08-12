use lexiroot_core::{
    Morpheme, MorphemeId, MorphemeKind, MorphemePositions, Provenance, WordDecomposition,
    MorphemeRef,
};

fn curated(evidence: &str) -> Provenance {
    Provenance::new(0.95, evidence).unwrap()
}

#[test]
fn provenance_rejects_confidence_out_of_range() {
    assert!(Provenance::new(-0.01, "x").is_err());
    assert!(Provenance::new(1.01, "x").is_err());
}

#[test]
fn provenance_accepts_boundary_confidence() {
    assert!(Provenance::new(0.0, "x").is_ok());
    assert!(Provenance::new(1.0, "x").is_ok());
}

#[test]
fn morpheme_id_is_deterministic_and_lowercased() {
    let a = MorphemeId::new("Spect");
    let b = MorphemeId::new("spect");
    assert_eq!(a, b);
    assert_eq!(a.as_str(), "spect");
}

/// A form attested in several positions is one morpheme carrying all of
/// them, not several morphemes each missing the others.
#[test]
fn positions_merge_and_serialize_canonically() {
    let mut positions = MorphemePositions::from_kind(MorphemeKind::Suffix);
    positions.insert(MorphemeKind::Prefix);
    assert!(positions.contains(MorphemeKind::Prefix));
    assert!(positions.contains(MorphemeKind::Suffix));
    assert!(!positions.contains(MorphemeKind::Root));
    // Always prefix/root/suffix order, whatever order they were inserted in,
    // so release databases stay byte-reproducible.
    assert_eq!(positions.to_storage_string(), "prefix|suffix");
    assert_eq!(MorphemePositions::parse("prefix|suffix").unwrap(), positions);
    assert!(MorphemePositions::parse("").is_err());
    assert!(MorphemePositions::parse("infix").is_err());
}

#[test]
fn morpheme_serde_round_trip() {
    let m = Morpheme::new(
        "spect",
        MorphemePositions::from_kind(MorphemeKind::Root),
        vec!["look".into(), "see".into()],
        curated("curated dataset entry 'spect'"),
    );
    let json = serde_json::to_string(&m).unwrap();
    let back: Morpheme = serde_json::from_str(&json).unwrap();
    assert_eq!(m, back);
}

#[test]
fn word_decomposition_serde_round_trip() {
    let root_id = MorphemeId::new("spect");
    let decomposition = WordDecomposition {
        word: "inspect".into(),
        segments: vec![
            MorphemeRef {
                morpheme_id: MorphemeId::new("in"),
                role: MorphemeKind::Prefix,
            },
            MorphemeRef {
                morpheme_id: root_id,
                role: MorphemeKind::Root,
            },
        ],
        provenance: curated("dataset example word for 'spect'"),
    };
    let json = serde_json::to_string(&decomposition).unwrap();
    let back: WordDecomposition = serde_json::from_str(&json).unwrap();
    assert_eq!(decomposition, back);
}

use serde::{Deserialize, Serialize};

/// Identifies which dataset/pass produced a given record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceId {
    ColinGoldbergMorphemes,
    WithEnglishWeCanRoots,
    /// LexiRoot's own hand-curated free-stem list. Both upstream datasets are
    /// Greek/Latin *bound-root* dictionaries, so the native Germanic core of
    /// English (`help`, `friend`, `believe`) is absent from them entirely and
    /// has to be supplied here.
    LexirootStems,
    /// Not read from a source dataset directly; derived at query time by
    /// the analyzer's algorithmic fallback.
    Inferred,
}

impl SourceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceId::ColinGoldbergMorphemes => "colingoldberg_morphemes",
            SourceId::WithEnglishWeCanRoots => "withenglishwecan_roots",
            SourceId::LexirootStems => "lexiroot_stems",
            SourceId::Inferred => "inferred",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "colingoldberg_morphemes" => Some(SourceId::ColinGoldbergMorphemes),
            "withenglishwecan_roots" => Some(SourceId::WithEnglishWeCanRoots),
            "lexiroot_stems" => Some(SourceId::LexirootStems),
            "inferred" => Some(SourceId::Inferred),
            _ => None,
        }
    }
}

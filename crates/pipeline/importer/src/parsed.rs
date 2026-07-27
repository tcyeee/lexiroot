use lexiroot_core::MorphemeKind;

/// A single morpheme as read from one source, before cross-source merging.
#[derive(Debug, Clone)]
pub struct ParsedMorpheme {
    pub form: String,
    pub kind: MorphemeKind,
    pub meanings: Vec<String>,
    pub evidence: String,
    pub examples: Vec<String>,
}

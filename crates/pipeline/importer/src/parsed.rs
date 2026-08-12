use lexiroot_core::MorphemeKind;

/// A single morpheme *sighting* as read from one source, before merging.
/// `kind` is the one position this sighting attests; sightings of the same
/// form are merged into a `MorphemePositions` set by `normalize`.
#[derive(Debug, Clone)]
pub struct ParsedMorpheme {
    pub form: String,
    pub kind: MorphemeKind,
    pub meanings: Vec<String>,
    pub evidence: String,
    pub examples: Vec<String>,
    /// Irregular surface allomorphs declared by the source. Only
    /// `lexiroot-stems` supplies these; the Greek/Latin root dictionaries
    /// have no field for them.
    pub variants: Vec<String>,
}

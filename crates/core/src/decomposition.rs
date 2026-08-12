use serde::{Deserialize, Serialize};

use crate::morpheme::{MorphemeId, MorphemeKind};
use crate::provenance::Provenance;

/// One segment of a word's decomposition, in left-to-right order. The
/// segment's form/meanings are looked up via `morpheme_id` rather than
/// duplicated here, so they can't drift from the `Morpheme` table.
///
/// `role` is which slot the morpheme fills *in this word*. It can't be read
/// off the `Morpheme` itself, because a form attested in several positions
/// (see `MorphemePositions`) only occupies one of them per word.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MorphemeRef {
    pub morpheme_id: MorphemeId,
    pub role: MorphemeKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordDecomposition {
    pub word: String,
    pub segments: Vec<MorphemeRef>,
    pub provenance: Provenance,
}

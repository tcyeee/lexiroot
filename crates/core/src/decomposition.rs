use serde::{Deserialize, Serialize};

use crate::morpheme::MorphemeId;
use crate::provenance::Provenance;

/// One segment of a word's decomposition, in left-to-right order. The
/// segment's form/meanings are looked up via `morpheme_id` rather than
/// duplicated here, so they can't drift from the `Morpheme` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MorphemeRef {
    pub morpheme_id: MorphemeId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WordDecomposition {
    pub word: String,
    pub segments: Vec<MorphemeRef>,
    pub provenance: Provenance,
}

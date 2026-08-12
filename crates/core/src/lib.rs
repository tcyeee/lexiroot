mod decomposition;
mod error;
mod morpheme;
mod provenance;
mod source;

pub use decomposition::{MorphemeRef, WordDecomposition};
pub use error::CoreError;
pub use morpheme::{Morpheme, MorphemeId, MorphemeKind, MorphemePositions};
pub use provenance::Provenance;
pub use source::SourceId;

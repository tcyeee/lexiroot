use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::provenance::Provenance;

/// Where a morpheme sits inside a word. This is a *position*, not a type:
/// the same form can legitimately occupy more than one of them (`hand` is a
/// free stem in `handbook` and a suffix in `beforehand`), which is why a
/// `Morpheme` carries a `MorphemePositions` set and an individual segment of
/// a decomposition carries the single `MorphemeKind` it filled *there*.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MorphemeKind {
    Prefix,
    Root,
    Suffix,
}

impl MorphemeKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            MorphemeKind::Prefix => "prefix",
            MorphemeKind::Root => "root",
            MorphemeKind::Suffix => "suffix",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "prefix" => Some(MorphemeKind::Prefix),
            "root" => Some(MorphemeKind::Root),
            "suffix" => Some(MorphemeKind::Suffix),
            _ => None,
        }
    }
}

/// The set of positions a single form is attested in.
///
/// Upstream sources record one row per (form, position) sighting. Treating
/// each sighting as a distinct morpheme is what previously kept `hand` — seen
/// only as `prefix` and `suffix` — out of the root table entirely, so no word
/// built on it could ever be segmented. Merging sightings into a position set
/// keeps one morpheme per form and lets the segmenter decide, per word, which
/// slot the form is actually filling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(into = "String", try_from = "String")]
pub struct MorphemePositions(u8);

impl MorphemePositions {
    pub const PREFIX: u8 = 1 << 0;
    pub const ROOT: u8 = 1 << 1;
    pub const SUFFIX: u8 = 1 << 2;

    pub fn empty() -> Self {
        Self(0)
    }

    pub fn from_kind(kind: MorphemeKind) -> Self {
        Self(Self::bit(kind))
    }

    fn bit(kind: MorphemeKind) -> u8 {
        match kind {
            MorphemeKind::Prefix => Self::PREFIX,
            MorphemeKind::Root => Self::ROOT,
            MorphemeKind::Suffix => Self::SUFFIX,
        }
    }

    pub fn insert(&mut self, kind: MorphemeKind) {
        self.0 |= Self::bit(kind);
    }

    pub fn union(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn contains(&self, kind: MorphemeKind) -> bool {
        self.0 & Self::bit(kind) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    /// Always emitted in prefix/root/suffix order so the serialized form is
    /// canonical — release databases must be byte-reproducible.
    pub fn iter(&self) -> impl Iterator<Item = MorphemeKind> + '_ {
        [MorphemeKind::Prefix, MorphemeKind::Root, MorphemeKind::Suffix]
            .into_iter()
            .filter(|k| self.contains(*k))
    }

    pub fn to_storage_string(self) -> String {
        self.iter().map(|k| k.as_str()).collect::<Vec<_>>().join("|")
    }

    pub fn parse(s: &str) -> Result<Self, CoreError> {
        let mut out = Self::empty();
        for part in s.split('|').filter(|p| !p.is_empty()) {
            let kind = MorphemeKind::parse(part)
                .ok_or_else(|| CoreError::InvalidMorphemePositions(s.to_string()))?;
            out.insert(kind);
        }
        if out.is_empty() {
            return Err(CoreError::InvalidMorphemePositions(s.to_string()));
        }
        Ok(out)
    }
}

impl From<MorphemePositions> for String {
    fn from(value: MorphemePositions) -> Self {
        value.to_storage_string()
    }
}

impl TryFrom<String> for MorphemePositions {
    type Error = CoreError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::parse(&value)
    }
}

/// A deterministic natural key — the lowercased form itself. Identity is the
/// form alone (not form + position), so the prefix sighting and the root
/// sighting of `dict` resolve to one morpheme carrying both positions.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MorphemeId(pub String);

impl MorphemeId {
    pub fn new(form: &str) -> Self {
        Self(form.to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Morpheme {
    pub id: MorphemeId,
    /// Original casing, e.g. "spect", "trans", "ation".
    pub form: String,
    pub positions: MorphemePositions,
    pub meanings: Vec<String>,
    /// Surface allomorphs of this morpheme: alternative spellings the form
    /// takes inside a word, which the segmenter accepts as this morpheme.
    ///
    /// Only *irregular* alternations belong here — ones no general rule
    /// predicts, so they have to be listed: Latin stem alternation
    /// (`admit` ~ `admiss`, `receive` ~ `recept`) and prefix assimilation
    /// (`in-` ~ `im-`, `il-`, `ir-`).
    ///
    /// Regular English spelling adjustments at a morpheme boundary —
    /// silent-e deletion (`believe` + `-able` -> `believable`), consonant
    /// doubling (`run` + `-ing`), `y` -> `i` (`happy` + `-ness`) — are
    /// deliberately NOT listed. They are productive, so enumerating them
    /// would mean writing three or four rows for every stem in the table;
    /// `lexiroot_analyzer::ortho` derives them instead.
    ///
    /// Empty for the overwhelming majority of morphemes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
    pub provenance: Provenance,
}

impl Morpheme {
    pub fn new(
        form: impl Into<String>,
        positions: MorphemePositions,
        meanings: Vec<String>,
        provenance: Provenance,
    ) -> Self {
        Self::with_variants(form, positions, meanings, Vec::new(), provenance)
    }

    pub fn with_variants(
        form: impl Into<String>,
        positions: MorphemePositions,
        meanings: Vec<String>,
        variants: Vec<String>,
        provenance: Provenance,
    ) -> Self {
        let form = form.into();
        let id = MorphemeId::new(&form);
        Self {
            id,
            form,
            positions,
            meanings,
            variants,
            provenance,
        }
    }
}

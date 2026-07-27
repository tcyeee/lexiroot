use serde::{Deserialize, Serialize};

use crate::provenance::Provenance;

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

/// A deterministic natural key ("{kind}:{form_lowercase}"), so identity
/// doesn't depend on processing order and stays stable across reproducible
/// exports.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MorphemeId(pub String);

impl MorphemeId {
    pub fn new(kind: MorphemeKind, form: &str) -> Self {
        Self(format!("{}:{}", kind.as_str(), form.to_lowercase()))
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
    pub kind: MorphemeKind,
    pub meanings: Vec<String>,
    pub provenance: Provenance,
}

impl Morpheme {
    pub fn new(form: impl Into<String>, kind: MorphemeKind, meanings: Vec<String>, provenance: Provenance) -> Self {
        let form = form.into();
        let id = MorphemeId::new(kind, &form);
        Self {
            id,
            form,
            kind,
            meanings,
            provenance,
        }
    }
}

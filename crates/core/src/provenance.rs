use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Every analysis result carries a confidence score and supporting evidence —
/// the "Explainable" design goal is modeled as a first-class struct, not
/// metadata bolted on later.
///
/// There is no source field. The dataset is a single curated file, so a
/// per-record source id had nothing left to distinguish; attribution for the
/// upstream datasets it was built from lives in `THIRD_PARTY_NOTICES.md`,
/// which is the whole of what their licenses require. What still separates a
/// curated entry from one the analyzer derived at query time is `confidence`
/// (0.95 vs 0.5) and the wording of `evidence`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    confidence: f32,
    pub evidence: String,
}

impl Provenance {
    pub fn new(confidence: f32, evidence: impl Into<String>) -> Result<Self, CoreError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(CoreError::ConfidenceOutOfRange(confidence));
        }
        Ok(Self {
            confidence,
            evidence: evidence.into(),
        })
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }
}

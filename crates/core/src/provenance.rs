use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::source::SourceId;

/// Every analysis result carries a confidence score, supporting evidence,
/// and a source reference — the "Explainable" design goal is modeled as a
/// first-class struct, not metadata bolted on later.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    pub source: SourceId,
    confidence: f32,
    pub evidence: String,
}

impl Provenance {
    pub fn new(source: SourceId, confidence: f32, evidence: impl Into<String>) -> Result<Self, CoreError> {
        if !(0.0..=1.0).contains(&confidence) {
            return Err(CoreError::ConfidenceOutOfRange(confidence));
        }
        Ok(Self {
            source,
            confidence,
            evidence: evidence.into(),
        })
    }

    pub fn confidence(&self) -> f32 {
        self.confidence
    }
}

use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum CoreError {
    #[error("confidence must be within 0.0..=1.0, got {0}")]
    ConfidenceOutOfRange(f32),
}

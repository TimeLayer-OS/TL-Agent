use std::fmt;

#[derive(Debug)]
pub enum SdkError {
    Io(std::io::Error),
    Json(serde_json::Error),
    MissingReceipt(String),
    InvalidReceipt { action_id: String, reason: String },
    TopologyViolation { action_id: String, reason: String },
    ScopeMismatch { action_id: String, detail: String },
    VerifierFailed { action_id: String, output: String },
    VerifierNotFound(String),
    RosterNotFound(String),
    BundleCorrupt(String),
    PolicyViolation(String),
}

impl fmt::Display for SdkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "IO error: {e}"),
            Self::Json(e) => write!(f, "JSON parse error: {e}"),
            Self::MissingReceipt(id) => write!(f, "NO_RECEIPT for action '{id}'"),
            Self::InvalidReceipt { action_id, reason } => {
                write!(f, "INVALID_RECEIPT for action '{action_id}': {reason}")
            }
            Self::TopologyViolation { action_id, reason } => {
                write!(f, "TOPOLOGY_DENIED for action '{action_id}': {reason}")
            }
            Self::ScopeMismatch { action_id, detail } => {
                write!(f, "SCOPE_MISMATCH for action '{action_id}': {detail}")
            }
            Self::VerifierFailed { action_id, output } => {
                write!(f, "TLSIG_INVALID for action '{action_id}': {output}")
            }
            Self::VerifierNotFound(path) => {
                write!(f, "timelayer-verifier not found at '{path}'")
            }
            Self::RosterNotFound(path) => write!(f, "roster not found at '{path}'"),
            Self::BundleCorrupt(msg) => write!(f, "bundle corrupt: {msg}"),
            Self::PolicyViolation(msg) => write!(f, "policy violation: {msg}"),
        }
    }
}

impl std::error::Error for SdkError {}

impl From<std::io::Error> for SdkError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for SdkError {
    fn from(e: serde_json::Error) -> Self {
        Self::Json(e)
    }
}

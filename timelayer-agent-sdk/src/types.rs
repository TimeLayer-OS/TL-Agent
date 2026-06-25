use serde::{Deserialize, Serialize};

// ---------- manifest.json ----------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    pub tl_agent_version: String,
    pub bundle_id: String,
    pub bundle_type: String,
    pub segment_id: Option<String>,
    pub owner_id: String,
    pub created_at: String,
    pub topology_id: String,
    pub receipt_count: usize,
    pub agent_can_write: bool,
    pub agent_can_issue_receipts: bool,
    pub no_receipt_no_action: bool,
    pub tlsig_roster: String,
    pub tlsig_k: u32,
    pub tlsig_mode: String,
    pub actions: Vec<String>,
}

// ---------- topology.json ----------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopologyNode {
    pub action_id: String,
    pub label: String,
    pub required_receipt_types: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopologyEdge {
    pub from: String,
    pub to: String,
    pub condition: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Topology {
    pub topology_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub mode: String,
    #[serde(default)]
    pub created_at: String,
    pub entry_action: String,
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
}

impl Topology {
    /// Returns all action_ids reachable from `from_action` in one step.
    pub fn next_actions(&self, from_action: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|e| e.from == from_action)
            .map(|e| e.to.clone())
            .collect()
    }

    /// Returns true if a transition from `from_action` to `to_action` is declared.
    pub fn allows_transition(&self, from_action: &str, to_action: &str) -> bool {
        self.edges
            .iter()
            .any(|e| e.from == from_action && e.to == to_action)
    }

    /// Returns the node for a given action_id, if it exists.
    pub fn node(&self, action_id: &str) -> Option<&TopologyNode> {
        self.nodes.iter().find(|n| n.action_id == action_id)
    }
}

// ---------- envelope.json ----------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Scope {
    pub paths: Vec<String>,
    pub read_only: bool,
    pub network_allowed: bool,
    pub write_allowed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Envelope {
    pub tl_agent_version: String,
    pub receipt_id: String,
    pub receipt_type: String,
    pub topology_id: String,
    pub action_id: String,
    pub label: String,
    pub issued_by: String,
    pub issued_at: String,
    pub valid_from: String,
    pub valid_until: Option<String>,
    pub status: String,
    pub previous_receipt_id: Option<String>,
    pub allowed_next_actions: Vec<String>,
    pub scope: Scope,
    pub tlsig_file: String,
    pub tlsig_workflow_id: String,
    pub tlsig_step_index: u64,
    pub tlsig_issuer: String,
    pub tlsig_roster_epoch: u64,
    pub tlsig_doc_digest: String,
}

impl Envelope {
    pub fn is_active(&self) -> bool {
        self.status == "active"
    }

    pub fn is_revoked(&self) -> bool {
        self.status == "revoked"
    }

    pub fn is_expired(&self) -> bool {
        self.status == "expired"
    }
}

// ---------- policies ----------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AgentPolicy {
    pub agent_can_issue_receipts: bool,
    pub agent_can_modify_receipts: bool,
    pub agent_can_modify_topology: bool,
    pub agent_can_request_next_receipt: bool,
    pub agent_can_execute_without_receipt: bool,
    pub agent_must_stop_on_missing_context: bool,
    pub agent_must_report_invalid_receipt: bool,
    pub text_is_not_proof: bool,
}

impl Default for AgentPolicy {
    fn default() -> Self {
        Self {
            agent_can_issue_receipts: false,
            agent_can_modify_receipts: false,
            agent_can_modify_topology: false,
            agent_can_request_next_receipt: true,
            agent_can_execute_without_receipt: false,
            agent_must_stop_on_missing_context: true,
            agent_must_report_invalid_receipt: true,
            text_is_not_proof: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StopPolicy {
    pub stop_on_missing_receipt: bool,
    pub stop_on_invalid_receipt: bool,
    pub stop_on_invalid_tlsig: bool,
    pub stop_on_scope_mismatch: bool,
    pub stop_on_context_uncertainty: bool,
    pub stop_on_topology_conflict: bool,
    pub stop_on_revoked_receipt: bool,
    pub stop_on_expired_receipt: bool,
    pub require_user_review_after_stop: bool,
}

impl Default for StopPolicy {
    fn default() -> Self {
        Self {
            stop_on_missing_receipt: true,
            stop_on_invalid_receipt: true,
            stop_on_invalid_tlsig: true,
            stop_on_scope_mismatch: true,
            stop_on_context_uncertainty: true,
            stop_on_topology_conflict: true,
            stop_on_revoked_receipt: true,
            stop_on_expired_receipt: true,
            require_user_review_after_stop: true,
        }
    }
}

// ---------- check result ----------

#[derive(Debug, Clone)]
pub enum CheckResult {
    Allow {
        action_id: String,
        receipt_id: String,
        allowed_next: Vec<String>,
    },
    Stop {
        action_id: String,
        reason: StopReason,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StopReason {
    NoReceipt,
    InvalidReceipt,
    TlsigInvalid,
    ReceiptRevoked,
    ReceiptExpired,
    TopologyDenied,
    ScopeMismatch,
    PolicyViolation,
    BundleCorrupt,
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoReceipt => write!(f, "NO_RECEIPT"),
            Self::InvalidReceipt => write!(f, "INVALID_RECEIPT"),
            Self::TlsigInvalid => write!(f, "TLSIG_INVALID"),
            Self::ReceiptRevoked => write!(f, "RECEIPT_REVOKED"),
            Self::ReceiptExpired => write!(f, "RECEIPT_EXPIRED"),
            Self::TopologyDenied => write!(f, "TOPOLOGY_DENIED"),
            Self::ScopeMismatch => write!(f, "SCOPE_MISMATCH"),
            Self::PolicyViolation => write!(f, "POLICY_VIOLATION"),
            Self::BundleCorrupt => write!(f, "BUNDLE_CORRUPT"),
        }
    }
}

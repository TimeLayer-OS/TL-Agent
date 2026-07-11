use serde::{Deserialize, Serialize};

// ---------- manifest.json ----------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Manifest {
    // The only fields the gate actually needs: which topology this bundle
    // belongs to, and the list of actions to load envelopes for.
    pub topology_id: String,
    pub actions: Vec<String>,

    // Everything below is informational metadata, defaulted so a bundle still
    // loads if a field is absent. The real proof is the cert+bundle pair the
    // verifier checks — not these fields. tlsig_roster/k/mode are vestigial:
    // the deployed timelayer-verifier embeds the roster and takes no such args.
    #[serde(default)]
    pub tl_agent_version: String,
    #[serde(default)]
    pub bundle_id: String,
    #[serde(default)]
    pub bundle_type: String,
    #[serde(default)]
    pub segment_id: Option<String>,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub receipt_count: usize,
    #[serde(default)]
    pub agent_can_write: bool,
    #[serde(default)]
    pub agent_can_issue_receipts: bool,
    #[serde(default)]
    pub no_receipt_no_action: bool,
    #[serde(default)]
    pub tlsig_roster: String,
    #[serde(default)]
    pub tlsig_k: u32,
    #[serde(default)]
    pub tlsig_mode: String,
}

// ---------- topology.json ----------

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TopologyNode {
    pub action_id: String,
    #[serde(default)]
    pub label: String,
    // Accept the cabinet's historical `required_receipts` key as an alias.
    #[serde(default, alias = "required_receipts")]
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
    #[serde(default)]
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

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Scope {
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub read_only: bool,
    #[serde(default)]
    pub network_allowed: bool,
    #[serde(default)]
    pub write_allowed: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Envelope {
    // Gate-relevant fields the loader cross-checks and the gate reads.
    pub receipt_id: String,
    pub topology_id: String,
    pub action_id: String,
    pub status: String,

    // Soft metadata — defaulted so an envelope still loads if absent.
    #[serde(default)]
    pub tl_agent_version: String,
    #[serde(default)]
    pub receipt_type: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub issued_by: String,
    #[serde(default)]
    pub issued_at: String,
    #[serde(default)]
    pub valid_from: String,
    #[serde(default)]
    pub valid_until: Option<String>,
    #[serde(default)]
    pub previous_receipt_id: Option<String>,
    #[serde(default)]
    pub allowed_next_actions: Vec<String>,
    #[serde(default)]
    pub scope: Scope,
    // tlsig_* metadata lives inside the cert/bundle binary; the SDK does not
    // require it duplicated in the envelope (path A — lightened schema).
    #[serde(default)]
    pub tlsig_file: String,
    #[serde(default)]
    pub tlsig_workflow_id: String,
    #[serde(default)]
    pub tlsig_step_index: u64,
    #[serde(default)]
    pub tlsig_issuer: String,
    #[serde(default)]
    pub tlsig_roster_epoch: u64,
    #[serde(default)]
    pub tlsig_doc_digest: String,
    /// Intent-commitment scheme this envelope's receipt attests.
    /// "tl-intent/1" = the receipt's subject is `intent_digest_v1()`, recomputed
    /// by the gate from the envelope's gate-relevant fields on every check.
    /// Empty = legacy envelope (see the binding policy in `check_action`).
    #[serde(default)]
    pub intent_scheme: String,
}

impl Envelope {
    /// Canonical intent commitment, scheme "tl-intent/1".
    ///
    /// This is the digest the receipt must attest (verifier `--expect`). It is
    /// RECOMPUTED from the envelope's gate-relevant fields on every check, never
    /// read from a mutable field: edit any committed field after issuance and
    /// the receipt stops matching (P0-01, receipt transplant).
    ///
    /// Canonical form: serde_json serialization with alphabetically sorted keys
    /// (serde_json's default Map ordering), no extra whitespace; absent optionals
    /// serialize as null. `status` is deliberately NOT committed — it is mutable
    /// lifecycle state; revocation lives in its own contour.
    pub fn intent_digest_v1(&self) -> String {
        use sha2::{Digest, Sha256};
        let canonical = serde_json::json!({
            "schema": "tl-intent/1",
            "topology_id": self.topology_id,
            "action_id": self.action_id,
            "receipt_id": self.receipt_id,
            "receipt_type": self.receipt_type,
            "valid_from": self.valid_from,
            "valid_until": self.valid_until,
            "previous_receipt_id": self.previous_receipt_id,
            "allowed_next_actions": self.allowed_next_actions,
            "scope": {
                "paths": self.scope.paths,
                "read_only": self.scope.read_only,
                "network_allowed": self.scope.network_allowed,
                "write_allowed": self.scope.write_allowed,
            },
        });
        let bytes = serde_json::to_vec(&canonical).expect("intent canonicalization");
        Sha256::digest(&bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

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
    /// Envelope declares no intent commitment: the receipt may be valid in
    /// itself, but nothing binds it to THIS action (P0-01).
    UnboundReceipt,
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
            Self::UnboundReceipt => write!(f, "UNBOUND_RECEIPT"),
        }
    }
}

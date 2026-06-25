use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::SdkError;
use crate::types::{AgentPolicy, Envelope, Manifest, StopPolicy, Topology};
use crate::verifier;

/// A loaded, validated agent bundle ready for action checking.
pub struct AgentBundle {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub topology: Topology,
    pub agent_policy: AgentPolicy,
    pub stop_policy: StopPolicy,
    /// action_id → (envelope, absolute path to proof.tlsig)
    pub(crate) envelopes: HashMap<String, (Envelope, PathBuf)>,
    /// absolute path to timelayer-verifier binary
    verifier_path: PathBuf,
    /// absolute path to roster.txt
    roster_path: PathBuf,
}

impl AgentBundle {
    /// Load a bundle from `bundle_dir`. Verifier is looked up:
    /// 1. Next to the binary (same dir as the calling process).
    /// 2. `bundle_dir/bin/timelayer-verifier`.
    /// 3. Path provided via `verifier_hint`.
    pub fn load(bundle_dir: impl AsRef<Path>) -> Result<Self, SdkError> {
        Self::load_with_verifier(bundle_dir, None)
    }

    pub fn load_with_verifier(
        bundle_dir: impl AsRef<Path>,
        verifier_hint: Option<&Path>,
    ) -> Result<Self, SdkError> {
        let root = bundle_dir.as_ref().to_path_buf();

        // manifest
        let manifest: Manifest = load_json(&root.join("manifest.json"))?;

        // topology (support both full and segment filenames)
        let topo_path = if root.join("topology.json").exists() {
            root.join("topology.json")
        } else if root.join("topology_segment.json").exists() {
            root.join("topology_segment.json")
        } else {
            return Err(SdkError::BundleCorrupt(
                "neither topology.json nor topology_segment.json found".into(),
            ));
        };
        let topology: Topology = load_json(&topo_path)?;

        // policies (optional — fall back to safe defaults)
        let agent_policy: AgentPolicy = load_json_or_default(&root.join("policies/agent_policy.json"));
        let stop_policy: StopPolicy = load_json_or_default(&root.join("policies/stop_policy.json"));

        // roster path (relative to bundle root as declared in manifest)
        let roster_path = root.join(&manifest.tlsig_roster);

        // verifier path resolution
        let verifier_path = resolve_verifier(&root, verifier_hint)?;

        // load all envelopes
        let receipts_dir = root.join("receipts");
        let mut envelopes = HashMap::new();
        for action_id in &manifest.actions {
            let action_dir = receipts_dir.join(action_id);
            let env_path = action_dir.join("envelope.json");
            let tlsig_path = action_dir.join("proof.tlsig");

            if !env_path.exists() {
                return Err(SdkError::BundleCorrupt(format!(
                    "envelope.json missing for action '{action_id}'"
                )));
            }
            if !tlsig_path.exists() {
                return Err(SdkError::BundleCorrupt(format!(
                    "proof.tlsig missing for action '{action_id}'"
                )));
            }

            let envelope: Envelope = load_json(&env_path)?;

            if envelope.action_id != *action_id {
                return Err(SdkError::BundleCorrupt(format!(
                    "envelope action_id mismatch: expected '{action_id}', got '{}'",
                    envelope.action_id
                )));
            }
            if envelope.topology_id != manifest.topology_id {
                return Err(SdkError::BundleCorrupt(format!(
                    "envelope topology_id mismatch for action '{action_id}'"
                )));
            }

            envelopes.insert(action_id.clone(), (envelope, tlsig_path));
        }

        Ok(Self {
            root,
            manifest,
            topology,
            agent_policy,
            stop_policy,
            envelopes,
            verifier_path,
            roster_path,
        })
    }

    /// Returns the envelope for an action without running any checks.
    pub fn envelope(&self, action_id: &str) -> Option<&Envelope> {
        self.envelopes.get(action_id).map(|(e, _)| e)
    }

    /// Returns the path to proof.tlsig for an action.
    pub fn tlsig_path(&self, action_id: &str) -> Option<&Path> {
        self.envelopes.get(action_id).map(|(_, p)| p.as_path())
    }

    /// Returns all action_ids in this bundle.
    pub fn action_ids(&self) -> Vec<&str> {
        self.manifest.actions.iter().map(String::as_str).collect()
    }

    /// Returns the topology's list of valid next actions from `action_id`.
    pub fn allowed_next(&self, action_id: &str) -> Vec<String> {
        self.topology.next_actions(action_id)
    }

    /// Checks if timelayer-verifier and roster are reachable.
    pub fn verifier_available(&self) -> bool {
        self.verifier_path.exists() && self.roster_path.exists()
    }

    pub(crate) fn verify_tlsig(&self, action_id: &str) -> Result<bool, SdkError> {
        let (_, tlsig_path) = self.envelopes.get(action_id).ok_or_else(|| {
            SdkError::MissingReceipt(action_id.to_string())
        })?;

        verifier::verify_tlsig(
            &self.verifier_path,
            tlsig_path,
            &self.roster_path,
            self.manifest.tlsig_k,
            &self.manifest.tlsig_mode,
        )
    }
}

// ---- helpers ----

fn load_json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, SdkError> {
    let text = std::fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

fn load_json_or_default<T: serde::de::DeserializeOwned + Default>(path: &Path) -> T {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

fn resolve_verifier(bundle_root: &Path, hint: Option<&Path>) -> Result<PathBuf, SdkError> {
    // 1. explicit hint
    if let Some(h) = hint {
        if h.exists() {
            return Ok(h.to_path_buf());
        }
    }

    // 2. next to the current executable
    if let Ok(exe) = std::env::current_exe() {
        let candidate = exe.parent().unwrap_or(Path::new(".")).join("timelayer-verifier");
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // 3. inside bundle/bin/
    let candidate = bundle_root.join("bin/timelayer-verifier");
    if candidate.exists() {
        return Ok(candidate);
    }

    // 4. on PATH — try resolving via `which`-style search
    if let Ok(output) = std::process::Command::new("sh")
        .args(["-c", "command -v timelayer-verifier"])
        .output()
    {
        let p = std::str::from_utf8(&output.stdout)
            .unwrap_or("")
            .trim()
            .to_string();
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }

    Err(SdkError::VerifierNotFound(
        "timelayer-verifier not found (set verifier_hint or place binary next to agent)".into(),
    ))
}

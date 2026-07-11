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
    /// action_id → (envelope, cert.tlcert path, bundle.tlbundle path)
    pub(crate) envelopes: HashMap<String, (Envelope, PathBuf, PathBuf)>,
    /// absolute path to timelayer-verifier binary
    verifier_path: PathBuf,
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

        // verifier path resolution
        let verifier_path = resolve_verifier(&root, verifier_hint)?;

        // load all envelopes. Each action carries a real notarial proof as two
        // binary blobs the deployed verifier checks together: cert.tlcert +
        // bundle.tlbundle (the exact pair the gateway's /v1/notarize returns).
        let receipts_dir = root.join("receipts");
        let mut envelopes = HashMap::new();
        for action_id in &manifest.actions {
            let action_dir = receipts_dir.join(action_id);
            let env_path = action_dir.join("envelope.json");
            let cert_path = action_dir.join("cert.tlcert");
            let bundle_path = action_dir.join("bundle.tlbundle");

            if !env_path.exists() {
                return Err(SdkError::BundleCorrupt(format!(
                    "envelope.json missing for action '{action_id}'"
                )));
            }
            if !cert_path.exists() {
                return Err(SdkError::BundleCorrupt(format!(
                    "cert.tlcert missing for action '{action_id}'"
                )));
            }
            if !bundle_path.exists() {
                return Err(SdkError::BundleCorrupt(format!(
                    "bundle.tlbundle missing for action '{action_id}'"
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

            envelopes.insert(action_id.clone(), (envelope, cert_path, bundle_path));
        }

        Ok(Self {
            root,
            manifest,
            topology,
            agent_policy,
            stop_policy,
            envelopes,
            verifier_path,
        })
    }

    /// Returns the envelope for an action without running any checks.
    pub fn envelope(&self, action_id: &str) -> Option<&Envelope> {
        self.envelopes.get(action_id).map(|(e, _, _)| e)
    }

    /// Returns the path to cert.tlcert for an action.
    pub fn cert_path(&self, action_id: &str) -> Option<&Path> {
        self.envelopes.get(action_id).map(|(_, c, _)| c.as_path())
    }

    /// Returns the path to bundle.tlbundle for an action.
    pub fn bundle_path(&self, action_id: &str) -> Option<&Path> {
        self.envelopes.get(action_id).map(|(_, _, b)| b.as_path())
    }

    /// Returns all action_ids in this bundle.
    pub fn action_ids(&self) -> Vec<&str> {
        self.manifest.actions.iter().map(String::as_str).collect()
    }

    /// Returns the topology's list of valid next actions from `action_id`.
    pub fn allowed_next(&self, action_id: &str) -> Vec<String> {
        self.topology.next_actions(action_id)
    }

    /// Checks if timelayer-verifier is reachable. The roster is embedded in the
    /// deployed verifier, so no separate roster file is required.
    pub fn verifier_available(&self) -> bool {
        self.verifier_path.exists()
    }

    pub(crate) fn verify_tlsig(&self, action_id: &str) -> Result<bool, SdkError> {
        let (_, cert_path, bundle_path) = self.envelopes.get(action_id).ok_or_else(|| {
            SdkError::MissingReceipt(action_id.to_string())
        })?;

        verifier::verify_tlsig(&self.verifier_path, cert_path, bundle_path)
    }

    /// Bound verification: the receipt must attest exactly `expected_hex`.
    pub(crate) fn verify_tlsig_bound(
        &self,
        action_id: &str,
        expected_hex: &str,
    ) -> Result<bool, SdkError> {
        let (_, cert_path, bundle_path) = self.envelopes.get(action_id).ok_or_else(|| {
            SdkError::MissingReceipt(action_id.to_string())
        })?;

        verifier::verify_tlsig_expect(&self.verifier_path, cert_path, bundle_path, expected_hex)
    }

    /// True when the installed verifier can bind receipts to subjects (--expect).
    pub fn verifier_supports_expect(&self) -> bool {
        verifier::verifier_supports_expect(&self.verifier_path)
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

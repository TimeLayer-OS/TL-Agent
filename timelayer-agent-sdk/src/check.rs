use crate::bundle::AgentBundle;
use crate::error::SdkError;
use crate::types::{CheckResult, StopReason};

impl AgentBundle {
    /// Main gate: run all checks for `action_id` and return Allow or Stop.
    ///
    /// Order follows the spec invariants (fail-closed at every step):
    ///   1. Receipt exists in bundle
    ///   2. Envelope is active (not revoked / expired)
    ///   3. Topology declares this action
    ///   4. cert.tlcert + bundle.tlbundle verify VALID offline (embedded roster)
    ///   5. → Allow
    pub fn check_action(&self, action_id: &str) -> CheckResult {
        // 1. Receipt present
        let envelope: &crate::types::Envelope = match self.envelopes.get(action_id).map(|(e, _, _)| e) {
            Some(e) => e,
            None => {
                return CheckResult::Stop {
                    action_id: action_id.to_string(),
                    reason: StopReason::NoReceipt,
                    message: format!("NO_RECEIPT: no receipt found for action '{action_id}'"),
                }
            }
        };

        // 2. Status check
        if envelope.is_revoked() {
            return CheckResult::Stop {
                action_id: action_id.to_string(),
                reason: StopReason::ReceiptRevoked,
                message: format!("RECEIPT_REVOKED: action '{action_id}' receipt is revoked"),
            };
        }
        if envelope.is_expired() {
            return CheckResult::Stop {
                action_id: action_id.to_string(),
                reason: StopReason::ReceiptExpired,
                message: format!("RECEIPT_EXPIRED: action '{action_id}' receipt is expired"),
            };
        }
        if !envelope.is_active() {
            return CheckResult::Stop {
                action_id: action_id.to_string(),
                reason: StopReason::InvalidReceipt,
                message: format!(
                    "INVALID_RECEIPT: action '{action_id}' has unexpected status '{}'",
                    envelope.status
                ),
            };
        }

        // 3. Topology declares this action
        if self.topology.node(action_id).is_none() {
            return CheckResult::Stop {
                action_id: action_id.to_string(),
                reason: StopReason::TopologyDenied,
                message: format!(
                    "TOPOLOGY_DENIED: action '{action_id}' not declared in topology"
                ),
            };
        }

        // 4. Verify .tlsig offline — BOUND to this action's intent.
        //
        // P0-01 (audit 2026-07-11, "receipt transplant"): a receipt that is valid
        // in itself proves nothing about THIS action. The binding policy, from
        // strongest to weakest, fail-closed at every fork:
        //   * intent_scheme == "tl-intent/1" — recompute the intent commitment
        //     from the envelope's gate-relevant fields and require the receipt
        //     to attest exactly it (--expect). Editing the envelope or
        //     transplanting a foreign receipt breaks the match.
        //   * legacy envelope with tlsig_doc_digest — bind to the declared
        //     digest. Weaker (the field travels with the envelope), kept for
        //     cabinet compatibility until issuers emit tl-intent/1.
        //   * legacy envelope with neither — STOP by default; accepting a
        //     receipt with no subject binding requires the explicit opt-out
        //     TL_AGENT_ALLOW_UNBOUND=1.
        //   * unknown intent_scheme — STOP (a newer protocol this SDK can't
        //     enforce must not silently degrade to unbound).
        let expected: Option<String> = match envelope.intent_scheme.as_str() {
            "tl-intent/1" => Some(envelope.intent_digest_v1()),
            "" => {
                if !envelope.tlsig_doc_digest.is_empty() {
                    Some(envelope.tlsig_doc_digest.clone())
                } else if std::env::var("TL_AGENT_ALLOW_UNBOUND").as_deref() == Ok("1") {
                    None // explicit legacy opt-out: pair validity only
                } else {
                    return CheckResult::Stop {
                        action_id: action_id.to_string(),
                        reason: StopReason::UnboundReceipt,
                        message: format!(
                            "UNBOUND_RECEIPT: action '{action_id}' declares no intent commitment \
                             (no intent_scheme, no tlsig_doc_digest) — a valid-in-itself receipt \
                             does not authorize THIS action. Re-issue the envelope with \
                             intent_scheme=tl-intent/1, or set TL_AGENT_ALLOW_UNBOUND=1 to accept \
                             legacy bundles (weaker guarantee)."
                        ),
                    };
                }
            }
            other => {
                return CheckResult::Stop {
                    action_id: action_id.to_string(),
                    reason: StopReason::UnboundReceipt,
                    message: format!(
                        "UNKNOWN_INTENT_SCHEME: '{other}' for action '{action_id}' — this SDK \
                         cannot enforce it, refusing rather than degrading to unbound (fail-closed)"
                    ),
                };
            }
        };

        if expected.is_some() && !self.verifier_supports_expect() {
            return CheckResult::Stop {
                action_id: action_id.to_string(),
                reason: StopReason::TlsigInvalid,
                message: format!(
                    "VERIFIER_NO_EXPECT: installed timelayer-verifier cannot bind receipts to \
                     subjects (--expect, v2.0.0+) — refusing to check action '{action_id}' \
                     unbound (fail-closed)"
                ),
            };
        }

        let verify_result = match &expected {
            Some(digest) => self.verify_tlsig_bound(action_id, digest),
            None => self.verify_tlsig(action_id),
        };
        match verify_result {
            Ok(true) => {} // valid (and bound, when a commitment was expected) — continue
            Ok(false) => {
                let detail = match &expected {
                    Some(d) => format!(
                        "receipt did not pass verification BOUND to intent digest {d} — \
                         either the pair is invalid, or it attests a different action \
                         (receipt transplant / edited envelope)"
                    ),
                    None => "notarial proof did not pass verification".to_string(),
                };
                return CheckResult::Stop {
                    action_id: action_id.to_string(),
                    reason: StopReason::TlsigInvalid,
                    message: format!("TLSIG_INVALID: action '{action_id}': {detail}"),
                };
            }
            Err(SdkError::VerifierNotFound(path)) => {
                return CheckResult::Stop {
                    action_id: action_id.to_string(),
                    reason: StopReason::BundleCorrupt,
                    message: format!("VERIFIER_NOT_FOUND: {path}"),
                };
            }
            Err(e) => {
                return CheckResult::Stop {
                    action_id: action_id.to_string(),
                    reason: StopReason::TlsigInvalid,
                    message: format!("TLSIG_CHECK_ERROR: {e}"),
                };
            }
        }

        // 5. All checks passed
        let allowed_next = envelope.allowed_next_actions.clone();
        CheckResult::Allow {
            action_id: action_id.to_string(),
            receipt_id: envelope.receipt_id.clone(),
            allowed_next,
        }
    }

    /// Checks whether a transition from `from_action` to `to_action` is allowed
    /// by topology. Does NOT verify the .tlsig of either action.
    pub fn check_transition(&self, from_action: &str, to_action: &str) -> CheckResult {
        if self.topology.allows_transition(from_action, to_action) {
            CheckResult::Allow {
                action_id: to_action.to_string(),
                receipt_id: String::new(),
                allowed_next: self.topology.next_actions(to_action),
            }
        } else {
            CheckResult::Stop {
                action_id: to_action.to_string(),
                reason: StopReason::TopologyDenied,
                message: format!(
                    "TOPOLOGY_DENIED: transition '{from_action}' → '{to_action}' not in topology"
                ),
            }
        }
    }

    /// Records that an action was executed. Writes a local JSON line to
    /// `bundle_root/execution_log.jsonl`. Does NOT issue a .tlsig — that is
    /// handled by the external receipt contour.
    pub fn record_execution(
        &self,
        action_id: &str,
        result_digest: &str,
    ) -> Result<(), SdkError> {
        use std::io::Write;

        let envelope = self
            .envelopes
            .get(action_id)
            .map(|(e, _, _)| e)
            .ok_or_else(|| SdkError::MissingReceipt(action_id.to_string()))?;

        let entry = serde_json::json!({
            "action_id": action_id,
            "receipt_id": envelope.receipt_id,
            "result_digest": result_digest,
            "recorded_at_utc": utc_now_approx(),
        });

        let log_path = self.root.join("execution_log.jsonl");
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        writeln!(file, "{}", entry)?;
        Ok(())
    }

    /// Runs `check_action` on every action in the bundle and returns a summary.
    pub fn audit(&self) -> Vec<AuditEntry> {
        self.manifest
            .actions
            .iter()
            .map(|id| {
                let result = self.check_action(id);
                AuditEntry {
                    action_id: id.clone(),
                    result,
                }
            })
            .collect()
    }
}

#[derive(Debug)]
pub struct AuditEntry {
    pub action_id: String,
    pub result: CheckResult,
}

impl AuditEntry {
    pub fn is_valid(&self) -> bool {
        matches!(self.result, CheckResult::Allow { .. })
    }
}

/// Returns a rough UTC timestamp string without pulling in chrono.
/// Format: "2026-06-24T00:00:00Z" (approximate, second precision from SystemTime).
fn utc_now_approx() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Simple ISO-8601 from unix timestamp (no leap seconds, UTC only)
    let s = secs % 60;
    let m = (secs / 60) % 60;
    let h = (secs / 3600) % 24;
    let total_days = secs / 86400;
    // Days since epoch to year/month/day (Gregorian, proleptic)
    let (year, month, day) = days_to_ymd(total_days);
    format!("{year:04}-{month:02}-{day:02}T{h:02}:{m:02}:{s:02}Z")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let dy = if is_leap(year) { 366 } else { 365 };
        if days < dy {
            break;
        }
        days -= dy;
        year += 1;
    }
    let leap = is_leap(year);
    let months = if leap {
        [31u64, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31u64, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };
    let mut month = 1u64;
    for dm in months {
        if days < dm {
            break;
        }
        days -= dm;
        month += 1;
    }
    (year, month, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

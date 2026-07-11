use std::path::Path;
use std::process::Command;

use crate::error::SdkError;

/// Decides validity from the verifier's exit status and stdout. Valid means the
/// process succeeded AND the trimmed stdout is EXACTLY "VALID FINAL" — nothing
/// else. A weaker verdict like "VALID PARTIAL" (even with exit code 0) is NOT
/// accepted: the gate requires the strongest, final verdict (fail-closed).
fn is_valid_final(success: bool, stdout: &str) -> bool {
    success && stdout.trim() == "VALID FINAL"
}

/// Calls `timelayer-verifier verify <cert.tlcert> <bundle.tlbundle>` and returns
/// true only on the exact "VALID FINAL" verdict. This matches the contract of the
/// deployed verifier binary: it takes BOTH the certificate and the bundle blob,
/// embeds its own roster, and signals a final valid receipt via exit code 0 + a
/// stdout line equal to "VALID FINAL". Anything else is invalid (fail-closed).
pub fn verify_tlsig(
    verifier_path: &Path,
    cert_path: &Path,
    bundle_path: &Path,
) -> Result<bool, SdkError> {
    if !verifier_path.exists() {
        return Err(SdkError::VerifierNotFound(
            verifier_path.display().to_string(),
        ));
    }

    let output = Command::new(verifier_path)
        .arg("verify")
        .arg(cert_path)
        .arg(bundle_path)
        .output()
        .map_err(|e| SdkError::Io(e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(is_valid_final(output.status.success(), &stdout))
}

/// Probes whether the installed verifier knows the `--expect` flag (v2.0.0+),
/// i.e. can cryptographically bind a receipt to an expected subject digest.
/// Any probe failure counts as "does not support" (fail-closed).
pub fn verifier_supports_expect(verifier_path: &Path) -> bool {
    Command::new(verifier_path)
        .args(["verify", "--help"])
        .output()
        .map(|o| {
            let text = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            text.contains("--expect")
        })
        .unwrap_or(false)
}

/// Like `verify_tlsig`, but additionally requires the receipt to attest exactly
/// `expected_hex` (the verifier's `--expect` flag). A receipt that is valid in
/// itself but attests a different subject returns Ok(false) — this is the
/// binding that stops receipt transplant (P0-01).
pub fn verify_tlsig_expect(
    verifier_path: &Path,
    cert_path: &Path,
    bundle_path: &Path,
    expected_hex: &str,
) -> Result<bool, SdkError> {
    if !verifier_path.exists() {
        return Err(SdkError::VerifierNotFound(
            verifier_path.display().to_string(),
        ));
    }

    let output = Command::new(verifier_path)
        .arg("verify")
        .arg(cert_path)
        .arg(bundle_path)
        .arg("--expect")
        .arg(expected_hex)
        .output()
        .map_err(SdkError::Io)?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    Ok(is_valid_final(output.status.success(), &stdout))
}

/// Returns the raw verifier output string for logging purposes.
pub fn verify_tlsig_verbose(
    verifier_path: &Path,
    cert_path: &Path,
    bundle_path: &Path,
) -> Result<String, SdkError> {
    if !verifier_path.exists() {
        return Err(SdkError::VerifierNotFound(
            verifier_path.display().to_string(),
        ));
    }

    let output = Command::new(verifier_path)
        .arg("verify")
        .arg(cert_path)
        .arg(bundle_path)
        .output()
        .map_err(|e| SdkError::Io(e))?;

    let out = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if out.is_empty() {
        Ok(String::from_utf8_lossy(&output.stderr).trim().to_string())
    } else {
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::is_valid_final;

    #[test]
    fn exact_valid_final_passes() {
        assert!(is_valid_final(true, "VALID FINAL"));
        assert!(is_valid_final(true, "  VALID FINAL\n")); // trimmed
    }

    #[test]
    fn valid_partial_with_exit_zero_is_false() {
        // The card's case: a "VALID PARTIAL" verdict on exit code 0 must NOT pass.
        assert!(!is_valid_final(true, "VALID PARTIAL"));
    }

    #[test]
    fn non_final_verdicts_are_false() {
        assert!(!is_valid_final(true, "NOT VALID"));
        assert!(!is_valid_final(true, "UNVERIFIABLE missing signature"));
        assert!(!is_valid_final(true, "VALID")); // not "VALID FINAL"
        assert!(!is_valid_final(true, "VALID FINAL extra")); // not exact
        assert!(!is_valid_final(true, ""));
    }

    #[test]
    fn valid_final_with_failure_exit_is_false() {
        assert!(!is_valid_final(false, "VALID FINAL"));
    }
}

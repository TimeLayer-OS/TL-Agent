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

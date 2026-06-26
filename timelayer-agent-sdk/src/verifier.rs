use std::path::Path;
use std::process::Command;

use crate::error::SdkError;

/// Calls `timelayer-verifier verify <cert.tlcert> <bundle.tlbundle>` and returns
/// true only on a VALID response. This matches the contract of the deployed
/// verifier binary: it takes BOTH the certificate and the bundle blob, embeds
/// its own roster, and signals validity via exit code 0 + a `VALID` line on
/// stdout (e.g. "VALID FINAL"). Anything else is treated as invalid (fail-closed).
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
    let line = stdout.trim();

    Ok(output.status.success() && line.starts_with("VALID"))
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

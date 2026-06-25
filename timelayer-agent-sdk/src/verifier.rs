use std::path::Path;
use std::process::Command;

use crate::error::SdkError;

/// Calls `timelayer-verifier verify <tlsig> <roster> <k> <mode>` and returns
/// true only on a VALID response. Any other outcome is treated as invalid
/// (fail-closed).
pub fn verify_tlsig(
    verifier_path: &Path,
    tlsig_path: &Path,
    roster_path: &Path,
    k: u32,
    mode: &str,
) -> Result<bool, SdkError> {
    if !verifier_path.exists() {
        return Err(SdkError::VerifierNotFound(
            verifier_path.display().to_string(),
        ));
    }
    if !roster_path.exists() {
        return Err(SdkError::RosterNotFound(
            roster_path.display().to_string(),
        ));
    }

    let output = Command::new(verifier_path)
        .arg("verify")
        .arg(tlsig_path)
        .arg(roster_path)
        .arg(k.to_string())
        .arg(mode)
        .output()
        .map_err(|e| SdkError::Io(e))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();

    Ok(line.starts_with("VALID"))
}

/// Returns the raw verifier output string for logging purposes.
pub fn verify_tlsig_verbose(
    verifier_path: &Path,
    tlsig_path: &Path,
    roster_path: &Path,
    k: u32,
    mode: &str,
) -> Result<String, SdkError> {
    if !verifier_path.exists() {
        return Err(SdkError::VerifierNotFound(
            verifier_path.display().to_string(),
        ));
    }

    let output = Command::new(verifier_path)
        .arg("verify")
        .arg(tlsig_path)
        .arg(roster_path)
        .arg(k.to_string())
        .arg(mode)
        .output()
        .map_err(|e| SdkError::Io(e))?;

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

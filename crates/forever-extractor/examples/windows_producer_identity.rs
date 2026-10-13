// SPDX-License-Identifier: GPL-3.0-or-later
// Authored 2026-10-11: explicit loaded-producer QA; no product enablement.

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    match forever_extractor::producer::loaded_identity_evidence() {
        Ok(identity) => {
            println!(
                "{{\"contract\":\"forever-windows-producer-seal-v1\",\"status\":\"accepted\",\"scope\":\"loaded_producer_content_identity\",\"file_sha256\":\"{}\",\"measured_payload_rva\":{},\"publisher_authenticated\":false}}",
                forever_casc::integrity::hex(&identity.file_sha256()),
                identity.payload_rva()
            );
            std::process::ExitCode::SUCCESS
        }
        Err(error) => {
            println!(
                "{{\"contract\":\"forever-windows-producer-seal-v1\",\"status\":\"failed\",\"code\":\"{}\"}}",
                error.code()
            );
            std::process::ExitCode::from(2)
        }
    }
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    println!(
        "{{\"contract\":\"forever-windows-producer-seal-v1\",\"status\":\"failed\",\"code\":\"producer_platform\"}}"
    );
    std::process::ExitCode::from(2)
}

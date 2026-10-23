//! Every behaviour spec section carries a confidence tag (`traced`,
//! `confirmed`, `static`, `guess`), so unverified claims stay visible.
//! Runs `tools/spec_confidence.py --check`; skips without python3.

use std::path::PathBuf;
use std::process::Command;

#[test]
fn every_spec_section_has_a_confidence_tag() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Ok(out) = Command::new("python3")
        .arg("tools/spec_confidence.py")
        .arg("--check")
        .current_dir(&root)
        .output()
    else {
        return;
    };
    assert!(
        out.status.success(),
        "untagged spec sections:\n{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

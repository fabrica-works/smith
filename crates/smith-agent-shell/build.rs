//! Build identity from the actual source commit; build tooling never runs in the service.

#[expect(clippy::disallowed_types, reason = "compile-time source metadata tooling is not a runtime process effect")]
fn git(arguments: &[&str]) -> String {
    use std::process::Command;

    let output = Command::new("git").args(arguments).output().expect("git supplies the source identity");
    assert!(output.status.success(), "git source identity query");
    String::from_utf8(output.stdout).expect("git metadata is UTF-8").trim().to_owned()
}

fn main() {
    let commit = git(&["rev-parse", "HEAD"]);
    let dirty = !git(&["status", "--porcelain=v1", "--untracked-files=normal"]).is_empty();
    println!("cargo:rustc-env=SMITH_BUILD={}{}", commit, if dirty { "-dirty" } else { "" });
    let head = git(&["rev-parse", "--git-path", "HEAD"]);
    println!("cargo:rerun-if-changed={head}");
    let contents = std::fs::read_to_string(head).expect("Git HEAD is readable at build time");
    if let Some(branch) = contents.trim().strip_prefix("ref: ") {
        println!("cargo:rerun-if-changed={}", git(&["rev-parse", "--git-path", branch]));
    }
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=Cargo.toml");
}

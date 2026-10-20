pub mod events;

use std::path::Path;

#[must_use]
pub fn unissued_auth() -> Vec<u8> {
    use base64::Engine;
    let claims = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b"{\"exp\":500}");
    serde_json::to_vec(&serde_json::json!({"auth_mode":"chatgpt","last_refresh":"2000-01-01T00:00:00Z",
        "tokens":{"id_token":"UNISSUED_ID","access_token":format!("header.{claims}.signature"),
        "account_id":"UNISSUED_ACCOUNT","refresh_token":"UNISSUED_REFRESH_MUST_NOT_COPY"}}))
    .expect("fixture")
}

#[must_use]
pub fn unexecuted_arm(directory: &Path, repository: &Path) -> smith_bench::arm::Build {
    use sha2::{Digest, Sha256};
    use smith_bench::arm::{ArmBinary, BuiltArm};
    // Replay fixtures select the product terminal invocation.
    let pin = smith_bench::read_configuration(&repository.join("benchmarks/agents/smith/standard.pin.toml"))
        .expect("standard Smith pin");
    let commit = pin.pin.minimum_commit.expect("minimum source identity");
    let ancestor = std::process::Command::new("git")
        .args(["-C", repository.to_str().expect("path"), "merge-base", "--is-ancestor", &commit, "HEAD"])
        .status()
        .expect("main ancestry");
    assert!(ancestor.success());
    let executable = b"unexecuted rendering fixture\n";
    let digest = format!("{:x}", Sha256::digest(executable));
    let path = directory.join(smith_bench::arm::arm_name(&commit, &digest).expect("fixture identity"));
    std::fs::create_dir(&path).expect("unexecuted arm directory");
    std::fs::write(path.join("smith"), executable).expect("unexecuted bytes");
    let arm = BuiltArm {
        version: 1,
        commit,
        sha256: digest.clone(),
        toolchain: "unexecuted test fixture".into(),
        binaries: vec![ArmBinary { name: "smith".into(), sha256: digest }],
    };
    std::fs::write(path.join("arm.json"), serde_json::to_vec(&arm).expect("manifest")).expect("fixture manifest");
    smith_bench::arm::read_build(&path.join("smith")).expect("manifest verification")
}

#[must_use]
pub fn calibration(task: &smith_bench::Task) -> smith_bench::Calibration {
    use sha2::{Digest, Sha256};
    smith_bench::Calibration {
        binary: "a".repeat(40),
        binary_sha256: "b".repeat(64),
        date: "2026-10-14".into(),
        summary: "summaries/calibration.json".into(),
        attempts: 10,
        passes: (1..=10).map(|index| format!("recorded-run/{index:02}")).collect(),
        deviations: vec![],
        control: "c".repeat(40),
        failed_before: vec!["control-run/01".into()],
        prompt_sha256: format!("{:x}", Sha256::digest(task.prompt.as_bytes())),
        seed_sha256: task.seed_sha256.clone(),
        checks_sha256: task.checks_sha256().expect("checks digest"),
    }
}

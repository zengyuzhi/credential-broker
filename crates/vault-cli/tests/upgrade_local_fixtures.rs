use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use serde_json::json;
use tempfile::TempDir;

const FIXTURE_ASSET_NAME: &str = "vault-aarch64-apple-darwin.tar.gz";

fn vault_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vault")
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
}

fn database_url(dir: &Path) -> String {
    format!("sqlite:{}?mode=rwc", dir.join("vault.db").display())
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn file_url(path: &Path) -> String {
    let absolute = path.canonicalize().expect("canonicalize fixture path");
    format!("file://{}", absolute.display())
}

fn make_executable(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("set executable bit");
    }
}

fn scratch_binary() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let binary = dir.path().join("vault");
    fs::copy(vault_bin(), &binary).expect("copy test binary");
    make_executable(&binary);
    (dir, binary)
}

fn write_release_fixture(
    dir: &Path,
    signature_path: &Path,
    tarball_path: &Path,
    version_tag: &str,
) -> PathBuf {
    let release_path = dir.join("release.json");
    let fixtures = fixture_dir();
    let body = json!({
        "tag_name": version_tag,
        "assets": [
            {
                "name": "SHA256SUMS",
                "browser_download_url": file_url(&fixtures.join("SHA256SUMS")),
            },
            {
                "name": "SHA256SUMS.minisig",
                "browser_download_url": file_url(signature_path),
            },
            {
                "name": FIXTURE_ASSET_NAME,
                "browser_download_url": file_url(tarball_path),
            }
        ]
    });
    fs::write(
        &release_path,
        serde_json::to_vec_pretty(&body).expect("serialize fixture release json"),
    )
    .expect("write fixture release json");
    release_path
}

fn run_upgrade(bin: &Path, state_root: &Path, release_path: &Path, args: &[&str]) -> Output {
    Command::new(bin)
        .args(args)
        .env("VAULT_DATABASE_URL", database_url(state_root))
        .env("VAULT_UPGRADE_TEST_RELEASE_JSON", release_path)
        .env(
            "VAULT_UPGRADE_TEST_PUBLIC_KEY_FILE",
            fixture_dir().join("test-pubkey.minisign"),
        )
        .env("VAULT_UPGRADE_TEST_ASSET_NAME", FIXTURE_ASSET_NAME)
        .output()
        .expect("run upgrade command")
}

fn write_pid_file(state_root: &Path, pid: u32) -> PathBuf {
    fs::create_dir_all(state_root).expect("create state directory");
    let pid_path = state_root.join("vault.pid");
    fs::write(&pid_path, pid.to_string()).expect("write pid file");
    pid_path
}

#[test]
fn upgrade_dry_run_should_verify_fixture_release_without_mutating_binary() {
    let (scratch, binary) = scratch_binary();
    let state_root = scratch.path().join("state");
    let original = fs::read(&binary).expect("read original binary");
    let fixtures = fixture_dir();
    let release_path = write_release_fixture(
        scratch.path(),
        &fixtures.join("SHA256SUMS.minisig"),
        &fixtures.join(FIXTURE_ASSET_NAME),
        "v0.1.2",
    );

    let output = run_upgrade(
        &binary,
        &state_root,
        &release_path,
        &["upgrade", "--dry-run"],
    );

    assert!(
        output.status.success(),
        "expected dry-run success, got: {}",
        combined_output(&output)
    );
    assert!(
        combined_output(&output).contains("would upgrade 0.1.1"),
        "expected dry-run output, got: {}",
        combined_output(&output)
    );
    assert_eq!(
        fs::read(&binary).expect("read upgraded binary"),
        original,
        "dry-run should not mutate the installed binary"
    );
}

#[test]
fn upgrade_should_fail_on_signature_mismatch_without_mutating_binary() {
    let (scratch, binary) = scratch_binary();
    let state_root = scratch.path().join("state");
    let original = fs::read(&binary).expect("read original binary");
    let fixtures = fixture_dir();
    let tampered_signature = scratch.path().join("SHA256SUMS.minisig");
    let mut signature =
        fs::read_to_string(fixtures.join("SHA256SUMS.minisig")).expect("read signature fixture");
    let last_char = signature.pop().expect("signature has content");
    signature.push(if last_char == 'A' { 'B' } else { 'A' });
    fs::write(&tampered_signature, signature).expect("write tampered signature");
    let release_path = write_release_fixture(
        scratch.path(),
        &tampered_signature,
        &fixtures.join(FIXTURE_ASSET_NAME),
        "v0.1.2",
    );

    let output = run_upgrade(
        &binary,
        &state_root,
        &release_path,
        &["upgrade", "--dry-run"],
    );

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected signature failure exit code, got: {}",
        combined_output(&output)
    );
    assert!(
        combined_output(&output).contains("signature verification failed:"),
        "expected signature failure output, got: {}",
        combined_output(&output)
    );
    assert_eq!(
        fs::read(&binary).expect("read binary after failure"),
        original,
        "signature failure should leave installed binary unchanged"
    );
}

#[test]
fn upgrade_should_fail_on_checksum_mismatch_without_mutating_binary() {
    let (scratch, binary) = scratch_binary();
    let state_root = scratch.path().join("state");
    let original = fs::read(&binary).expect("read original binary");
    let fixtures = fixture_dir();
    let tampered_tarball = scratch.path().join("tampered.tar.gz");
    fs::write(&tampered_tarball, "not the signed tarball").expect("write tampered tarball");
    let release_path = write_release_fixture(
        scratch.path(),
        &fixtures.join("SHA256SUMS.minisig"),
        &tampered_tarball,
        "v0.1.2",
    );

    let output = run_upgrade(
        &binary,
        &state_root,
        &release_path,
        &["upgrade", "--dry-run"],
    );

    assert_eq!(
        output.status.code(),
        Some(3),
        "expected checksum failure exit code, got: {}",
        combined_output(&output)
    );
    assert!(
        combined_output(&output).contains("checksum mismatch for"),
        "expected checksum failure output, got: {}",
        combined_output(&output)
    );
    assert_eq!(
        fs::read(&binary).expect("read binary after checksum failure"),
        original,
        "checksum failure should leave installed binary unchanged"
    );
}

#[test]
fn upgrade_should_refuse_when_pid_file_points_to_a_live_process() {
    let (scratch, binary) = scratch_binary();
    let state_root = scratch.path().join("state");
    let release_path = scratch.path().join("missing-release.json");
    let pid_path = write_pid_file(&state_root, std::process::id());

    let output = run_upgrade(&binary, &state_root, &release_path, &["upgrade", "--check"]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected live-pid refusal exit code, got: {}",
        combined_output(&output)
    );
    assert!(
        combined_output(&output).contains("vault daemon is running"),
        "expected daemon refusal message, got: {}",
        combined_output(&output)
    );
    assert!(
        combined_output(&output).contains("vault serve stop"),
        "expected stop hint, got: {}",
        combined_output(&output)
    );
    assert!(
        pid_path.exists(),
        "live pid file should remain in place after refusal"
    );
}

#[test]
fn upgrade_should_ignore_and_delete_a_stale_pid_file() {
    let (scratch, binary) = scratch_binary();
    let state_root = scratch.path().join("state");
    let fixtures = fixture_dir();
    let release_path = write_release_fixture(
        scratch.path(),
        &fixtures.join("SHA256SUMS.minisig"),
        &fixtures.join(FIXTURE_ASSET_NAME),
        "v0.1.2",
    );
    let pid_path = write_pid_file(&state_root, 999_999);

    let output = run_upgrade(&binary, &state_root, &release_path, &["upgrade", "--check"]);

    assert!(
        output.status.success(),
        "expected stale pid to be ignored, got: {}",
        combined_output(&output)
    );
    assert!(
        combined_output(&output).contains("update available: 0.1.1 → 0.1.2"),
        "expected upgrade check output, got: {}",
        combined_output(&output)
    );
    assert!(
        !pid_path.exists(),
        "stale pid file should be removed after the check"
    );
}

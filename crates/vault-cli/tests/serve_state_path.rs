use std::{
    fs,
    path::Path,
    process::{Command, Output},
    sync::atomic::{AtomicU16, Ordering},
};

use tempfile::TempDir;

fn vault_bin() -> &'static str {
    env!("CARGO_BIN_EXE_vault")
}

fn reserve_port() -> u16 {
    static NEXT_PORT: AtomicU16 = AtomicU16::new(39000);
    NEXT_PORT.fetch_add(1, Ordering::Relaxed)
}

fn database_url(dir: &Path) -> String {
    format!("sqlite:{}?mode=rwc", dir.join("vault.db").display())
}

fn run_vault(args: &[&str], cwd: &Path, db_url: &str) -> Output {
    Command::new(vault_bin())
        .args(args)
        .current_dir(cwd)
        .env("VAULT_DATABASE_URL", db_url)
        .output()
        .expect("run vault command")
}

fn combined_output(output: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

#[test]
fn serve_status_should_find_background_server_from_another_cwd() {
    let state = TempDir::new().expect("state tempdir");
    let cwd_one = TempDir::new().expect("cwd one");
    let cwd_two = TempDir::new().expect("cwd two");
    let port = reserve_port();
    let port_string = port.to_string();
    let db_url = database_url(state.path());

    let start = run_vault(
        &["serve", "--port", &port_string, "--background"],
        cwd_one.path(),
        &db_url,
    );
    assert!(
        start.status.success(),
        "start failed: {}",
        combined_output(&start)
    );
    let pid_path = state.path().join("vault.pid");
    let pid_metadata = fs::metadata(&pid_path).expect("pid file metadata");
    assert!(
        pid_path.is_file(),
        "expected canonical pid file at {}",
        pid_path.display()
    );
    assert!(
        !cwd_one.path().join(".local/vault.pid").exists(),
        "did not expect a cwd-relative pid file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        assert_eq!(pid_metadata.permissions().mode() & 0o777, 0o600);
    }

    let status = run_vault(
        &["serve", "--port", &port_string, "status"],
        cwd_two.path(),
        &db_url,
    );

    let _ = run_vault(
        &["serve", "--port", &port_string, "stop"],
        cwd_one.path(),
        &db_url,
    );

    assert!(
        status.status.success(),
        "status failed: {}",
        combined_output(&status)
    );
    assert!(
        combined_output(&status).contains("Vault server is running"),
        "expected running status, got: {}",
        combined_output(&status)
    );
}

#[test]
fn upgrade_check_should_refuse_when_background_server_was_started_from_another_cwd() {
    let state = TempDir::new().expect("state tempdir");
    let cwd_one = TempDir::new().expect("cwd one");
    let cwd_two = TempDir::new().expect("cwd two");
    let port = reserve_port();
    let port_string = port.to_string();
    let db_url = database_url(state.path());

    let start = run_vault(
        &["serve", "--port", &port_string, "--background"],
        cwd_one.path(),
        &db_url,
    );
    assert!(
        start.status.success(),
        "start failed: {}",
        combined_output(&start)
    );

    let upgrade = run_vault(&["upgrade", "--check"], cwd_two.path(), &db_url);

    let _ = run_vault(
        &["serve", "--port", &port_string, "stop"],
        cwd_one.path(),
        &db_url,
    );

    assert_eq!(
        upgrade.status.code(),
        Some(2),
        "expected daemon refusal exit code, got output: {}",
        combined_output(&upgrade)
    );
    assert!(
        combined_output(&upgrade).contains("vault daemon is running"),
        "expected daemon refusal message, got: {}",
        combined_output(&upgrade)
    );
}

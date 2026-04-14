use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use anyhow::{Context, Result, bail};
use clap::Args;
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;

const RELEASES_API_BASE: &str = "https://api.github.com/repos/zengyuzhi/credential-broker";

static EMBEDDED_PUBLIC_KEY: OnceLock<PublicKey> = OnceLock::new();
static EMBEDDED_PUBLIC_KEY_ID: OnceLock<String> = OnceLock::new();

#[derive(Debug, Args)]
#[command(about = "Check for and install a newer vault binary")]
pub struct UpgradeCommand {
    #[arg(
        long,
        help = "Check whether a newer release is available without installing"
    )]
    pub check: bool,

    #[arg(
        long,
        value_name = "VERSION",
        help = "Upgrade or roll back to a specific version"
    )]
    pub to: Option<String>,

    #[arg(
        long,
        help = "Allow re-installing the same or an older version when used with --to"
    )]
    pub force: bool,

    #[arg(long, help = "Run verification without replacing the current binary")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Version {
    major: u64,
    minor: u64,
    patch: u64,
}

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

struct StagingDir {
    path: PathBuf,
}

#[cfg(debug_assertions)]
fn test_override(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(not(debug_assertions))]
fn test_override(_name: &str) -> Option<String> {
    None
}

impl StagingDir {
    fn create(current_exe: &Path) -> Result<Self> {
        let install_dir = current_exe
            .parent()
            .context("current executable has no parent directory")?;
        let path = install_dir.join(format!(".vault-upgrade-{}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("failed to remove existing {}", path.display()))?;
        }
        fs::create_dir(&path)
            .with_context(|| format!("failed to create staging directory {}", path.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            fs::set_permissions(&path, fs::Permissions::from_mode(0o700))
                .with_context(|| format!("failed to set permissions on {}", path.display()))?;
        }
        Ok(Self { path })
    }

    fn join(&self, child: &str) -> PathBuf {
        self.path.join(child)
    }
}

impl Drop for StagingDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

fn extract_pubkey_base64(pubkey_contents: &str) -> String {
    pubkey_contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("untrusted comment:"))
        .expect("release public key should contain a minisign base64 line")
        .to_string()
}

fn current_pubkey_contents() -> String {
    if let Some(path) = test_override("VAULT_UPGRADE_TEST_PUBLIC_KEY_FILE") {
        return fs::read_to_string(&path)
            .unwrap_or_else(|_| panic!("release public key should be readable from {}", path));
    }

    std::str::from_utf8(include_bytes!("../../release-pubkey.minisign"))
        .expect("release public key should be valid UTF-8")
        .to_string()
}

fn embedded_public_key() -> &'static PublicKey {
    EMBEDDED_PUBLIC_KEY.get_or_init(|| {
        PublicKey::from_base64(&extract_pubkey_base64(&current_pubkey_contents()))
            .expect("release public key should parse at runtime")
    })
}

fn embedded_public_key_id() -> &'static str {
    EMBEDDED_PUBLIC_KEY_ID.get_or_init(|| extract_pubkey_base64(&current_pubkey_contents()))
}

fn parse_version(input: &str) -> Result<Version> {
    let trimmed = input.trim().trim_start_matches('v');
    let (core, _suffix) = trimmed.split_once('-').unwrap_or((trimmed, ""));
    let mut parts = core.split('.');
    let major = parts
        .next()
        .context("missing major version")?
        .parse()
        .context("invalid major version")?;
    let minor = parts
        .next()
        .context("missing minor version")?
        .parse()
        .context("invalid minor version")?;
    let patch = parts
        .next()
        .context("missing patch version")?
        .parse()
        .context("invalid patch version")?;

    if parts.next().is_some() {
        bail!("version must be MAJOR.MINOR.PATCH");
    }

    Ok(Version {
        major,
        minor,
        patch,
    })
}

fn normalize_release_tag(version: &str) -> String {
    if version.trim().starts_with('v') {
        version.trim().to_string()
    } else {
        format!("v{}", version.trim())
    }
}

fn enforce_force_requires_target(cmd: &UpgradeCommand) -> Result<()> {
    if cmd.force && cmd.to.is_none() {
        bail!("`--force` requires `--to <version>`");
    }
    Ok(())
}

fn enforce_version_guard(
    running: &Version,
    target: &Version,
    force: bool,
    explicit_target: Option<&str>,
) -> Result<()> {
    if target > running {
        return Ok(());
    }

    if force && explicit_target.is_some() {
        return Ok(());
    }

    let target_text = target.to_string();
    bail!(
        "refusing to downgrade {} → {} without --force --to {}",
        running,
        target,
        target_text
    );
}

fn target_triple() -> Result<&'static str> {
    match std::env::consts::ARCH {
        "aarch64" => Ok("aarch64-apple-darwin"),
        "x86_64" => Ok("x86_64-apple-darwin"),
        other => bail!("unsupported architecture: {}", other),
    }
}

fn target_asset_name() -> Result<String> {
    if let Some(asset_name) = test_override("VAULT_UPGRADE_TEST_ASSET_NAME") {
        return Ok(asset_name);
    }

    Ok(format!("vault-{}.tar.gz", target_triple()?))
}

fn ensure_install_path_supported(current_exe: &Path) -> Result<()> {
    let path = current_exe.display().to_string();
    if path.starts_with("/opt/homebrew/Cellar") || path.starts_with("/usr/local/Cellar") {
        bail!(
            "refusing to self-upgrade a Homebrew-managed vault binary at {}. Use `brew upgrade` instead.",
            current_exe.display()
        );
    }
    if path.starts_with("/Library/Frameworks") || path.contains(".app/Contents/MacOS/") {
        bail!(
            "refusing to self-upgrade a system-managed vault binary at {}. Reinstall or update it with the package manager or app bundle that installed it.",
            current_exe.display()
        );
    }
    Ok(())
}

async fn fetch_release(cmd: &UpgradeCommand) -> Result<GitHubRelease> {
    if let Some(path) = test_override("VAULT_UPGRADE_TEST_RELEASE_JSON") {
        let body = fs::read_to_string(&path)
            .with_context(|| format!("failed to read test release metadata from {}", path))?;
        return serde_json::from_str(&body)
            .with_context(|| format!("failed to parse test release metadata from {}", path));
    }

    let endpoint = match cmd.to.as_deref() {
        Some(version) => format!(
            "{RELEASES_API_BASE}/releases/tags/{}",
            normalize_release_tag(version)
        ),
        None => format!("{RELEASES_API_BASE}/releases/latest"),
    };

    let response = reqwest::Client::new()
        .get(endpoint)
        .header(reqwest::header::ACCEPT, "application/vnd.github+json")
        .header(
            reqwest::header::USER_AGENT,
            format!("vault-cli/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .context("failed to query GitHub releases API")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("release lookup failed ({}): {}", status, body);
    }

    response
        .json()
        .await
        .context("failed to parse release metadata")
}

fn find_asset_url<'a>(release: &'a GitHubRelease, asset_name: &str) -> Result<&'a str> {
    release
        .assets
        .iter()
        .find(|asset| asset.name == asset_name)
        .map(|asset| asset.browser_download_url.as_str())
        .with_context(|| format!("release did not contain asset {}", asset_name))
}

async fn download_to_file(url: &str, destination: &Path) -> Result<()> {
    if let Ok(parsed) = reqwest::Url::parse(url)
        && parsed.scheme() == "file"
    {
        let source = parsed
            .to_file_path()
            .map_err(|_| anyhow::anyhow!("invalid file URL: {}", url))?;
        tokio::fs::copy(&source, destination)
            .await
            .with_context(|| {
                format!(
                    "failed to copy local asset {} to {}",
                    source.display(),
                    destination.display()
                )
            })?;
        return Ok(());
    }

    let response = reqwest::Client::new()
        .get(url)
        .header(
            reqwest::header::USER_AGENT,
            format!("vault-cli/{}", env!("CARGO_PKG_VERSION")),
        )
        .send()
        .await
        .with_context(|| format!("failed to download {}", url))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("download failed for {} ({}): {}", url, status, body);
    }

    let bytes = response
        .bytes()
        .await
        .context("failed to read download body")?;
    tokio::fs::write(destination, bytes)
        .await
        .with_context(|| format!("failed to write {}", destination.display()))
}

fn verify_signed_manifest(
    public_key: &PublicKey,
    manifest_path: &Path,
    signature_path: &Path,
) -> Result<()> {
    let manifest = fs::read(manifest_path)
        .with_context(|| format!("failed to read {}", manifest_path.display()))?;
    let signature = Signature::from_file(signature_path).with_context(|| {
        format!(
            "signature verification failed: failed to read {}",
            signature_path.display()
        )
    })?;
    public_key
        .verify(&manifest, &signature, false)
        .map_err(|err| anyhow::anyhow!("signature verification failed: {}", err))
}

fn expected_checksum_for_asset(manifest_contents: &str, asset_name: &str) -> Result<String> {
    manifest_contents
        .lines()
        .find_map(|line| {
            let (sha, name) = line.split_once("  ")?;
            (name.trim() == asset_name).then(|| sha.trim().to_string())
        })
        .with_context(|| format!("missing checksum entry for {}", asset_name))
}

fn compute_sha256(path: &Path) -> Result<String> {
    let output = Command::new("shasum")
        .args(["-a", "256"])
        .arg(path)
        .output()
        .with_context(|| format!("failed to compute SHA-256 for {}", path.display()))?;
    if !output.status.success() {
        bail!(
            "failed to compute SHA-256 for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout = String::from_utf8(output.stdout).context("invalid shasum output")?;
    stdout
        .split_whitespace()
        .next()
        .map(|sha| sha.to_string())
        .context("shasum did not return a checksum")
}

fn extract_binary_from_tarball(tarball_path: &Path, extract_dir: &Path) -> Result<PathBuf> {
    fs::create_dir_all(extract_dir)
        .with_context(|| format!("failed to create {}", extract_dir.display()))?;
    let output = Command::new("tar")
        .args(["-xzf"])
        .arg(tarball_path)
        .args(["-C"])
        .arg(extract_dir)
        .output()
        .with_context(|| format!("failed to extract {}", tarball_path.display()))?;
    if !output.status.success() {
        bail!(
            "failed to extract {}: {}",
            tarball_path.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let binary_path = extract_dir.join("vault");
    let metadata = fs::metadata(&binary_path)
        .with_context(|| format!("expected extracted binary at {}", binary_path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        if metadata.permissions().mode() & 0o111 == 0 {
            bail!("extracted binary is not executable");
        }
    }

    Ok(binary_path)
}

fn is_verification_failure(message: &str) -> bool {
    message.starts_with("signature verification failed:")
        || message.starts_with("checksum mismatch")
}

async fn execute_upgrade(
    cmd: &UpgradeCommand,
    running: &Version,
    target: &Version,
    release: &GitHubRelease,
) -> Result<()> {
    let canonical_exe = std::env::current_exe()
        .context("failed to resolve current executable path")?
        .canonicalize()
        .context("failed to canonicalize current executable path")?;

    ensure_install_path_supported(&canonical_exe)?;
    let staging = StagingDir::create(&canonical_exe)?;
    let asset_name = target_asset_name()?;
    let manifest_path = staging.join("SHA256SUMS");
    let signature_path = staging.join("SHA256SUMS.minisig");
    let tarball_path = staging.join(&asset_name);
    let extract_dir = staging.join("extract");
    let staged_binary = staging.join("vault.new");

    download_to_file(find_asset_url(release, "SHA256SUMS")?, &manifest_path).await?;
    download_to_file(
        find_asset_url(release, "SHA256SUMS.minisig")?,
        &signature_path,
    )
    .await?;
    verify_signed_manifest(embedded_public_key(), &manifest_path, &signature_path)?;

    download_to_file(find_asset_url(release, &asset_name)?, &tarball_path).await?;
    let manifest_contents =
        fs::read_to_string(&manifest_path).context("failed to read checksum manifest")?;
    let expected_sha = expected_checksum_for_asset(&manifest_contents, &asset_name)?;
    let actual_sha = compute_sha256(&tarball_path)?;
    if expected_sha != actual_sha {
        bail!(
            "checksum mismatch for {}: expected {}, got {}",
            asset_name,
            expected_sha,
            actual_sha
        );
    }

    let extracted_binary = extract_binary_from_tarball(&tarball_path, &extract_dir)?;
    fs::rename(&extracted_binary, &staged_binary).with_context(|| {
        format!(
            "failed to stage extracted binary {}",
            extracted_binary.display()
        )
    })?;

    if cmd.dry_run {
        println!(
            "would upgrade {} → {} (checksum OK, signature OK by key {})",
            running,
            target,
            embedded_public_key_id()
        );
        return Ok(());
    }

    fs::rename(&staged_binary, &canonical_exe).with_context(|| {
        format!(
            "failed to atomically replace installed binary {}",
            canonical_exe.display()
        )
    })?;
    println!("upgraded {} → {}", running, target);
    Ok(())
}

fn platform_guard() {
    if std::env::consts::OS != "macos" {
        eprintln!(
            "error: vault upgrade is only supported on macOS. See https://github.com/zengyuzhi/credential-broker for details."
        );
        std::process::exit(5);
    }
}

pub async fn run_upgrade_command(cmd: UpgradeCommand) -> Result<()> {
    let _ = embedded_public_key();
    eprintln!("signing key: {}", embedded_public_key_id());

    platform_guard();
    enforce_force_requires_target(&cmd)?;

    if let Some(pid) = crate::commands::serve::running_background_server_pid() {
        eprintln!("error: vault daemon is running (pid {}).", pid);
        eprintln!("hint:  stop it first with `vault serve stop`, then retry `vault upgrade`.");
        eprintln!("       after upgrading, restart with `vault serve --background`.");
        std::process::exit(2);
    }

    let running = parse_version(env!("CARGO_PKG_VERSION"))?;
    let release = fetch_release(&cmd).await?;
    let target = parse_version(&release.tag_name)?;

    if cmd.check {
        if target > running {
            println!("update available: {} → {}", running, target);
        } else {
            println!("already on latest version: {}", running);
        }
        return Ok(());
    }

    if let Err(err) = enforce_version_guard(&running, &target, cmd.force, cmd.to.as_deref()) {
        eprintln!("error: {err}");
        std::process::exit(4);
    }

    if let Err(err) = execute_upgrade(&cmd, &running, &target, &release).await {
        if is_verification_failure(&err.to_string()) {
            eprintln!("{err}");
            std::process::exit(3);
        }
        return Err(err);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use tempfile::TempDir;

    use super::{
        PublicKey, UpgradeCommand, Version, compute_sha256, embedded_public_key_id,
        enforce_version_guard, expected_checksum_for_asset, extract_binary_from_tarball,
        parse_version, verify_signed_manifest,
    };

    fn fixture_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
    }

    #[test]
    fn parse_version_should_strip_v_prefix() {
        let parsed = parse_version("v1.2.3").expect("parse version");
        assert_eq!(
            parsed,
            Version {
                major: 1,
                minor: 2,
                patch: 3,
            }
        );
    }

    #[test]
    fn version_guard_should_reject_equal_without_force() {
        let running = parse_version("0.2.5").expect("running version");
        let target = parse_version("0.2.5").expect("target version");

        let err = enforce_version_guard(&running, &target, false, None).expect_err("guard failure");

        assert_eq!(
            err.to_string(),
            "refusing to downgrade 0.2.5 → 0.2.5 without --force --to 0.2.5"
        );
    }

    #[test]
    fn version_guard_should_accept_equal_with_force_and_explicit_target() {
        let running = parse_version("0.2.5").expect("running version");
        let target = parse_version("0.2.5").expect("target version");

        enforce_version_guard(&running, &target, true, Some("0.2.5")).expect("guard passes");
    }

    #[test]
    fn embedded_public_key_id_should_match_generated_key() {
        assert_eq!(
            embedded_public_key_id(),
            "RWQjSf1Hgz33cGQ+jv/tnwd0/puGVn1N1Xw0wddNdPRL7L2w4avYdVMt"
        );
    }

    #[test]
    fn force_flag_without_target_should_be_invalid() {
        let cmd = UpgradeCommand {
            check: false,
            to: None,
            force: true,
            dry_run: false,
        };

        let err = super::enforce_force_requires_target(&cmd).expect_err("force should fail");
        assert_eq!(err.to_string(), "`--force` requires `--to <version>`");
    }

    #[test]
    fn signed_manifest_fixture_should_verify() {
        let fixtures = fixture_dir();
        let pubkey_contents =
            fs::read_to_string(fixtures.join("test-pubkey.minisign")).expect("read fixture pubkey");
        let public_key = PublicKey::decode(&pubkey_contents).expect("decode pubkey");

        verify_signed_manifest(
            &public_key,
            &fixtures.join("SHA256SUMS"),
            &fixtures.join("SHA256SUMS.minisig"),
        )
        .expect("fixture signature should verify");
    }

    #[test]
    fn signed_manifest_should_fail_when_manifest_is_tampered() {
        let fixtures = fixture_dir();
        let pubkey_contents =
            fs::read_to_string(fixtures.join("test-pubkey.minisign")).expect("read fixture pubkey");
        let public_key = PublicKey::decode(&pubkey_contents).expect("decode pubkey");
        let tempdir = TempDir::new().expect("tempdir");
        let tampered_manifest = tempdir.path().join("SHA256SUMS");
        let original = fs::read_to_string(fixtures.join("SHA256SUMS")).expect("read manifest");
        fs::write(
            &tampered_manifest,
            original.replace("vault-aarch64-apple-darwin.tar.gz", "tampered.tar.gz"),
        )
        .expect("write tampered manifest");

        let err = verify_signed_manifest(
            &public_key,
            &tampered_manifest,
            &fixtures.join("SHA256SUMS.minisig"),
        )
        .expect_err("tampered manifest should fail");

        assert!(
            err.to_string()
                .starts_with("signature verification failed:")
        );
    }

    #[test]
    fn checksum_lookup_and_computation_should_match_fixture() {
        let fixtures = fixture_dir();
        let manifest = fs::read_to_string(fixtures.join("SHA256SUMS")).expect("read manifest");
        let asset_name = "vault-aarch64-apple-darwin.tar.gz";

        let expected = expected_checksum_for_asset(&manifest, asset_name).expect("lookup checksum");
        let actual =
            compute_sha256(&fixtures.join(asset_name)).expect("compute fixture tarball checksum");

        assert_eq!(expected, actual);
    }

    #[test]
    fn extract_tarball_should_produce_executable_binary() {
        let fixtures = fixture_dir();
        let tempdir = TempDir::new().expect("tempdir");

        let extracted = extract_binary_from_tarball(
            &fixtures.join("vault-aarch64-apple-darwin.tar.gz"),
            tempdir.path(),
        )
        .expect("extract tarball");

        assert!(extracted.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let mode = fs::metadata(&extracted)
                .expect("metadata")
                .permissions()
                .mode();
            assert_ne!(mode & 0o111, 0);
        }
    }
}

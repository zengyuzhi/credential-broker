use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=release-pubkey.minisign");

    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));
    let pubkey_path = manifest_dir.join("release-pubkey.minisign");
    let contents = fs::read_to_string(&pubkey_path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", pubkey_path.display()));
    let base64 = contents
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("untrusted comment:"))
        .unwrap_or_else(|| {
            panic!(
                "{} did not contain a minisign public key",
                pubkey_path.display()
            )
        });

    minisign_verify::PublicKey::from_base64(base64).unwrap_or_else(|err| {
        panic!(
            "invalid minisign public key in {}: {err}",
            pubkey_path.display()
        )
    });
}

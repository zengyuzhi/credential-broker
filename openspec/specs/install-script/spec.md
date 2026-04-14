# Spec: install-script

## Purpose

Provide a one-command install script that downloads the correct prebuilt binary from GitHub Releases and installs it to the user's PATH.

## Requirements

### Requirement: One-command install script
A bash install script SHALL download the correct prebuilt binary from GitHub Releases and install it to the user's PATH.

#### Scenario: Install on macOS Apple Silicon
- **WHEN** user runs `curl -fsSL <raw-url>/install.sh | bash` on an M1/M2/M3 Mac
- **THEN** the script detects `aarch64` architecture
- **AND** downloads `vault-aarch64-apple-darwin.tar.gz` from the latest GitHub Release
- **AND** extracts the `vault` binary to `~/.local/bin/` (or `/usr/local/bin/` with sudo)
- **AND** prints "vault installed to <path>. Run 'vault --help' to get started."

#### Scenario: Install on macOS Intel
- **WHEN** user runs the install script on an Intel Mac
- **THEN** the script detects `x86_64` architecture and downloads the correct binary

#### Scenario: Non-macOS platform
- **WHEN** user runs the install script on Linux or Windows
- **THEN** the script prints "credential-broker requires macOS (Keychain integration). See README for details." and exits

#### Scenario: PATH guidance
- **WHEN** the install directory is not in the user's PATH
- **THEN** the script prints instructions to add it (e.g. `export PATH="$HOME/.local/bin:$PATH"`)

#### Scenario: Upgrade
- **WHEN** user runs the install script and `vault` is already installed
- **THEN** the script replaces the existing binary with the new version
- **AND** prints "vault upgraded to vX.Y.Z"

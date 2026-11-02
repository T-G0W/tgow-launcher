# T-GOW Launcher

One place for every T-GOW project on GitHub. The launcher finds the right package for your system, installs it without administrator rights and keeps the launcher and every app up to date as soon as a new release is published.

Written in Rust with egui. Windows 10 and 11, Linux.

## Features

- Catalog of every T-G0W repository with the `tgow-launcher` topic and a signed release
- Detects operating system, real CPU architecture and Linux distribution and picks the matching asset
- Installs per user: portable builds, MSI, Inno Setup, NSIS, AppImage, tar.gz, Flatpak
- Automatic updates for the launcher and all apps, with rollback
- Every release manifest is checked against an Ed25519 signature before anything is installed

## Privacy

No telemetry, no account, no tracking. The launcher only talks to GitHub. GitHub sees your IP address when the launcher checks for updates.

## Building

```
cargo xtask dist
```

This builds the release binary with every local path remapped and fails if the binary still contains a home directory path or an email address. A plain `cargo build --release` works too but keeps the paths of your machine in panic messages.

Checks used in CI:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask nocomments
cargo test --workspace
cargo deny check
```

## License

MIT or Apache-2.0, at your option. Fonts: Chakra Petch, Bai Jamjuree and JetBrains Mono under the SIL Open Font License 1.1.

Made by T-GOW · https://github.com/T-G0W

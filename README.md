# T-GOW Launcher

One place for every T-GOW project on GitHub. The launcher finds the right package for your system, installs it without administrator rights and keeps the launcher and every app up to date as soon as a new release is published.

Written in Rust with egui. Windows 10 and 11, Linux.

## Features

- Catalog of every T-G0W repository with the `tgow-launcher` topic and a signed release
- Detects operating system, real CPU architecture and Linux distribution and picks the matching asset
- Installs per user: portable builds, MSI, Inno Setup, NSIS, AppImage, tar.gz, Flatpak
- Automatic updates for the launcher and all apps, with rollback
- Every release manifest is checked against an Ed25519 signature before anything is installed

## Adding a project

A repository of T-G0W shows up in the launcher when it has the `tgow-launcher` topic and its latest release carries a signed `tgow-release.json`.

Name every asset `<app>-<version>-<os>-<arch>[-setup|-portable].<ext>`, for example `ferrovox-1.1.0-windows-x64-setup.exe`, `ferrovox-1.1.0-windows-arm64-portable.zip` or `ferrovox-1.1.0-linux-x86_64.AppImage`. The launcher picks the asset for the native architecture first and falls back to emulation.

Call the reusable workflow from the tag workflow once the build jobs have uploaded their artifacts and the GitHub release for the tag exists:

```yaml
release:
  needs: build
  uses: T-G0W/tgow-launcher/.github/workflows/tgow-release.yml@<commit>
  with:
    app: ferrovox
    name: Ferrovox
    artifacts: "*"
    installer: inno
  secrets:
    TGOW_SIGNING_KEY: ${{ secrets.TGOW_SIGNING_KEY }}
```

It writes `tgow-release.json`, signs it and uploads it with `SHA256SUMS.txt` to the release. The same steps run locally with `cargo run -p tgow-release -- manifest ...` and `sign`.

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
cargo xtask licenses --check
cargo deny check
```

## License

MIT or Apache-2.0, at your option. Fonts: Chakra Petch, Bai Jamjuree and JetBrains Mono under the SIL Open Font License 1.1. The licenses of all bundled crates are collected in `THIRD-PARTY-LICENSES.txt`, generated with `cargo xtask licenses`.

Made by T-GOW · https://github.com/T-G0W

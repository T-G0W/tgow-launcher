# Security policy

T-GOW Launcher downloads and installs software, so security reports get priority over everything else.

## Reporting a vulnerability

Please do not open a public issue. Use **Security › Report a vulnerability** on this repository instead, which reaches T-GOW privately.

Helpful details:

- what an attacker can do and what they need for it, for example control over a network, a GitHub account or a local user
- steps or a small proof of concept
- the launcher version from **About** and your system

T-GOW aims to answer within a few days. Once a fix is released, the report can be published with credit if you like.

## Supported versions

Only the latest release gets security fixes. The launcher installs its own updates automatically by default.

## What the launcher promises

- Nothing is installed or started unless the release manifest carries a valid Ed25519 signature from a key built into the launcher.
- Every downloaded file must match the size and SHA-256 in that signed manifest.
- Network access goes to five GitHub hosts only, over TLS, with every redirect checked against the same list. Images from repositories are only decoded, never run.
- Archives cannot write outside their target folder.
- A failed self-update leaves the previous version in place.

A way around any of these is a vulnerability. Problems that need an already compromised computer or a stolen signing key are out of scope, but still welcome as hardening ideas.

# Toolgate

Toolgate is a local-first macOS activity gate for AI agents. v0.1 evaluates and records actions delivered through a verified adapter; it is **not** universal operating-system interception.

## Quick start

```sh
cargo build --workspace
cargo run -p toolgate-daemon -- /tmp/toolgate.sock
TOOLGATE_SOCKET=/tmp/toolgate.sock cargo run -p toolgate-hook -- claude-code pre < hook-event.json
cd Packages/App && swift build
```

Install hooks into a chosen Claude settings file with `toolgate-hook install /path/settings.json /absolute/path/toolgate-hook`; remove only Toolgate-owned groups with `toolgate-hook uninstall /path/settings.json`.

The daemon is user-scoped and requires neither root, Full Disk Access, Endpoint Security, nor Network Extension. See [capability matrix](docs/CAPABILITY_MATRIX.md) before relying on an adapter.

## Quality gates

`make check` runs formatting, Clippy with warnings denied, Rust tests, and the Swift build. The active developer directory on this machine is Command Line Tools-only, so XCTest is unavailable until full Xcode is selected.

# Architecture

The v0.1 process boundary is a per-user Rust daemon over a Unix `SOCK_STREAM`, using a 4-byte big-endian length prefix and versioned JSON. Frames are limited to 1 MiB. The SwiftUI menu-bar app is a client; it does not open SQLite directly. SQLite uses WAL and receives only redacted tool input.

The only implemented enforcement surface is a Claude Code `PreToolUse` command hook. The hook fails closed on malformed input, daemon error, disconnect, or a five-second timeout. Post-tool events are observation only.

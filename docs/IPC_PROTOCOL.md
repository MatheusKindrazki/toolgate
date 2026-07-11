# IPC protocol

Each connection is a `SOCK_STREAM` with a u32 big-endian length prefix followed by UTF-8 JSON. Messages carry `version: 1`, optional `id`, `type`, and `params`; frames over 1 MiB are rejected. `health` returns daemon state and capability coverage. `evaluate` returns action, state, and a redacted representation.

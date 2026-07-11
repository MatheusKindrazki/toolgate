# Capability matrix

| Adapter | State | Contract / limitation |
|---|---|---|
| Claude Code 2.1.202 PreToolUse | enforced | Locally verified against current documented nested hook schema. `permissionDecision: deny` is supported; daemon failures use exit 2 fail-closed. |
| Claude Code PostToolUse | observed | Cannot undo a completed tool call. |
| Codex | unsupported | No hook/wrapper event contract encoded; do not infer a sandbox contract. |
| Hermes | unsupported | Public synchronous hook contract has not been verified. |
| OS file/network interception | unsupported | Intentionally excluded from v0.1. |

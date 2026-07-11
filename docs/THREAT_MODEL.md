# Threat model

Toolgate protects only actions mediated by its installed adapter. Agent processes and IPC payloads are untrusted. Private socket placement and bounded framing reduce accidental exposure, but another process under the same macOS UID is not a security boundary.

Secrets are redacted recursively by key before the database insert. Toolgate never requests root, Full Disk Access, Endpoint Security, or Network Extension. It does not claim to block filesystem reads, network traffic, or process launches outside adapter mediation.

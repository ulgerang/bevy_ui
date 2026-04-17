# ADR: Asset Loading Boundary

Status: Accepted for Stage 9 checkpoint.

## Decision

The crate remains string-first for XML and JSON styles. Image `src` and default font paths are handed to Bevy `AssetServer`; asset identity, loading state, failures, hot reload, and dependency tracking are Bevy-owned.

No custom `AssetLoader`, document handles, hot reload support, async loading, or asset dependency graph is introduced in this stage.

## Drivers

- The current public API loads layout/style strings.
- Bevy already owns image/font asset handles.
- AssetLoader support would introduce document identity and reload semantics that deserve a separate design.

## Consequences

- Examples may show asset paths for images/fonts, but the crate does not claim asset lifecycle management.
- Missing/invalid assets are handled according to Bevy `AssetServer` behavior.
- Future Bevy asset integration requires a separate ADR.

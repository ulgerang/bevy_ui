# ADR: Render Effects Boundary

Status: Accepted for Stage 10.

## Decision

`borderRadius`, `boxShadow`, `filter`, and `backdropFilter` remain unsupported render effects. They are preserved as spawn-time `UiXmlUnsupportedEffects` metadata for diagnostics and future renderer work.

Runtime hover/active/disabled restyling updates Bevy-supported UI properties such as `Style`, colors, and `Outline`. It does not update or remove `UiXmlUnsupportedEffects`.

## Drivers

- Bevy UI 0.12.1 does not provide browser-compatible rendering for these effects.
- Custom material or render pipeline work would be larger than this form-control slice.
- Public docs must not imply visual support for metadata-only properties.

## Alternatives Considered

- Continue metadata-only: chosen.
- Map a native subset now: deferred until a smaller supported subset is identified.
- Add a custom UI renderer: rejected for this stage.
- Wait for a Bevy upgrade: possible future path.

## Consequences

- Unsupported effect values remain inspectable.
- State-specific unsupported effects are not live render components.
- Any future visual support requires a renderer ADR and tests.

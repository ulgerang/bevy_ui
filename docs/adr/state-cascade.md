# ADR: Runtime State Cascade

## Decision

Runtime state styling uses a bounded Bevy-owned state model instead of a full browser CSS engine.

The base style is computed from selector rules by specificity and JSON source order. Runtime pseudo-selector rules only participate when the crate has a concrete runtime state source. Nested JSON state blocks such as `hover`, `active`, `focus`, and `disabled` are Bevy-specific overlays attached to the computed style, not selectors.

For supported runtime states, the merge order is:

1. Base selector rules by specificity and source order.
2. Supported runtime pseudo-selector rules by the same selector cascade.
3. Inline XML attributes as base-layer overrides.
4. Nested JSON state overlays for the active state.

Within nested overlays, disabled wins over every other state. Focus applies before active/hover overlays so focused decorations can remain visible unless active/hover explicitly override the same field.

## Drivers

- Existing JSON state blocks already work for button `Interaction` styling.
- `:disabled` has static XML-attribute semantics for computed styles and component-owned runtime semantics after spawn.
- Bevy `Interaction` provides reliable hover and press state, while `UiXmlFocus` provides the crate-owned focus source.
- Existing users should not lose the ergonomic nested JSON state authoring style.

## Alternatives Considered

- Full CSS-like dynamic selector matching: rejected for the next implementation slice because it requires retained ancestry, invalidation, and broad restyle scheduling beyond the current MVP.
- Nested JSON overlays only: rejected as the final direction because parsed pseudo selectors would remain misleading and forms/focus would need another architecture pass.
- Metadata-only preservation: retained for unsupported features such as full form serialization and custom render effects until their ADR gates are approved.

## Why Chosen

This path makes hover, active, and disabled real Bevy runtime states while keeping the implementation bounded. It also preserves JSON-first authoring and leaves room for fuller dynamic selectors later.

## Consequences

- A mutable disabled component becomes the runtime source of truth after spawn.
- XML `disabled` seeds the runtime disabled component but does not remain authoritative.
- `:focus` and nested `focus` are applied when `UiXmlFocus.entity` points at the entity and the entity is not disabled.
- Runtime selector matching requires retained entity context before pseudo selectors can be recomputed safely.

## Follow-Ups

- Add retained runtime selector context before broader dynamic pseudo matching.
- Keep keyboard focus traversal behind a separate ADR.
- Keep render effects behind a separate ADR and one bounded implementation slice.

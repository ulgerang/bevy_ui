# ADR: Runtime State Cascade

## Decision

Runtime state styling uses a bounded Bevy-owned state model instead of a full browser CSS engine.

The base style is computed from selector rules by specificity and JSON source order. Runtime pseudo-selector rules only participate when the crate has a concrete runtime state source. Nested JSON state blocks such as `hover`, `active`, `focus`, and `disabled` are Bevy-specific overlays attached to the computed style, not selectors.

Supported terminal runtime pseudo-states are `:hover`, `:active`, `:focus`,
`:focus-visible`, `:focus-within`, `:checked`, and `:disabled`. Matching is
terminal for direct state overlays and ancestor-state selector chains for
retained entity context: `button:checked`, `panel:focus-within`,
`.form:focus-within .field`, and `.tabs:checked > .panel` are supported.

For supported runtime states, the merge order is:

1. Base selector rules by specificity and source order.
2. Supported runtime pseudo-selector rules by the same selector cascade.
3. Inline XML attributes as base-layer overrides.
4. Nested JSON state overlays for the active state.

The resolved runtime overlay order is base, `checked`, `focusWithin`,
`focus`/`focusVisible`, then `active` or `hover`. `disabled` short-circuits and
overrides all non-disabled overlays. `:focus-visible` follows
`UiXmlInputModality` so pointer focus can differ from keyboard-visible focus.

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

This path makes hover, active, focus, focus-visible, focus-within, checked, and
disabled real Bevy runtime states while keeping the implementation bounded. It
also preserves JSON-first authoring and leaves room for fuller dynamic selectors
later.

## Consequences

- A mutable disabled component becomes the runtime source of truth after spawn.
- XML `disabled` seeds the runtime disabled component but does not remain authoritative.
- `:focus` and nested `focus` are applied when `UiXmlFocus.entity` points at the entity and the entity is not disabled.
- `:checked` follows `UiXmlChecked` after spawn, not the XML `checked`
  attribute snapshot.
- `:focus-within` follows `UiXmlFocus.entity` for the focused entity and its
  retained entity ancestors.
- Ancestor-state selector-chain overlays are precomputed at spawn from retained
  selector context and activated from runtime focus/checked state.

## Follow-Ups

- Broader CSSOM-style invalidation remains deferred.
- Keep keyboard focus traversal behind a separate ADR.
- Keep render effects behind a separate ADR and one bounded implementation slice.

# ADR: CSS-Like Feature Parity Boundary

Status: Accepted for CSS-like feature parity slice.

## Decision

Extend the existing JSON stylesheet, selector, runtime-state, and text-input
architecture while adding a bounded native CSS rule parser, `::placeholder`,
and Bevy-native dynamic ancestor-state selector restyling. Do not add CSSOM or
custom renderer scope.

This slice supports selector groups, CSS attribute operators, terminal and
ancestor-state runtime pseudo-states, bounded shorthands, JSON-native and
`::placeholder` styling, expanded color/length parsing, focus-visible modality,
text cursor editing, form events, and runtime metadata where values map
deterministically to Bevy UI 0.12.

## Supported Now

- JSON style maps remain supported, and bounded native CSS rule blocks are also accepted.
- Selector groups split top-level commas and keep each member's specificity.
- Attribute operators `[attr]`, `[attr=value]`, `~=`, `|=`, `^=`, `$=`, and
  `*=` use documented string matching.
- Terminal runtime pseudo-states include `:checked`, `:focus-within`, and
  `:focus-visible` alongside existing hover/active/focus/disabled states.
- Ancestor-state selector chains such as `.form:focus-within .field` and
  `.tabs:checked > .panel` are supported from retained entity context.
- `inset`, bounded `flex`, border side widths, and font family/weight/style
  metadata are parsed.
- Text input placeholders use XML `placeholder` plus nested JSON `placeholder`
  style or CSS `::placeholder`.
- Hex, `rgb()`/`rgba()`, selected named colors, and accepted length forms map to
  Bevy values or documented fallbacks.

## Deferred

- Full CSSOM and arbitrary browser selector invalidation.
- Renderer work for real gradients, shadows, filters, backdrop filters, border
  radius, and side-specific border colors.
- OS clipboard, IME/composition, text selection, and browser validation UI.

## Consequences

The crate offers a broader CSS-like authoring subset while preserving Bevy-owned
runtime sources of truth. Public docs must describe this as bounded CSS-like JSON
support, not browser CSS parity.

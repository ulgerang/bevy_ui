# ADR: Game CSS Tokens, States, And Transitions

## Decision

`bevy_ui_xml` supports a bounded, game-oriented CSS layer for theme tokens,
runtime pseudo-states, and lightweight transitions. This is not a full CSSOM or
browser animation engine.

## Supported

- Custom properties such as `--accent` in JSON or native CSS style maps.
- `var(--name)` and `var(--name, fallback)` resolution.
- `UiXmlThemeTokens` as the Bevy resource shape for active theme tokens; parser
  entry points can resolve with an explicit token map.
- Runtime pseudo-states `:selected`, `:open`, `:valid`, and `:invalid`, plus
  nested `selected`, `open`, `valid`, and `invalid` style blocks.
- `UiXmlSelected` and `UiXmlOpen` reusable state components for future widgets.
- Bounded `transition` parsing for color/opacity-oriented properties. Runtime
  interpolation currently applies to background/opacity through headless-safe ECS
  updates.

## Deferred

- Full CSS custom-property inheritance and registration (`@property`).
- CSSOM, cascade layers, and `!important`.
- Browser keyframes/animation engine.
- Transition interpolation for every CSS property.

## Consequences

Game UI styles can share theme tokens, use reusable runtime state hooks, and get
simple visual polish without importing a browser engine model. Future widgets
should reuse `UiXmlSelected` and `UiXmlOpen` instead of defining duplicate state.

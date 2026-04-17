# ADR: Focus Source Of Truth

Status: Accepted and implemented for the focus runtime slice.

## Decision

Runtime `:focus` styling is enabled through the crate-owned `UiXmlFocus` resource. `UiXmlFocus.entity` is the source of truth for the one focused entity in a Bevy world.

`UiXmlRuntimeState.focused` is derived state used by the style runtime. External code should set `UiXmlFocus.entity`, not mutate `UiXmlRuntimeState.focused` directly.

## Drivers

- Bevy UI does not provide browser focus semantics by default, so this crate needs an explicit source of truth before applying `:focus`.
- Keyboard/gamepad navigation, pointer focus, accessibility focus, and user-owned focus can disagree.
- Enabling `:focus` prematurely would make public runtime behavior hard to change.

## Alternatives Considered

- Crate-owned focus resource: chosen. It provides a stable source of truth without claiming keyboard/gamepad navigation semantics.
- Bevy interaction approximation: rejected because hover/press is not focus.
- External integration hook: viable later by writing `UiXmlFocus.entity`.
- Metadata-only preservation: rejected for this slice because a bounded source of truth is now available.

## Consequences

- `:focus` selectors and nested `focus` blocks can affect runtime styles when `UiXmlFocus.entity` points at the entity.
- Only one entity can be focused through the resource.
- Disabled entities do not become effectively focused even if the resource still points at them.
- Checkbox/radio behavior does not depend on focus.

## Follow-Ups

- Define keyboard/gamepad navigation semantics in a future ADR.
- Define accessibility integration in a future ADR if needed.

# ADR: Text Input Runtime

Status: Accepted for Stage 7.

## Decision

`<input type="text">` and `<input>` are executable text controls. Other input
types remain structural metadata until a bounded runtime contract is defined for
them.

Text controls use Bevy components as runtime sources of truth:

- `UiXmlTextInput` marks executable text controls.
- `UiXmlTextValue(pub String)` owns the mutable value after spawn.
- `UiXmlTextDisplay(pub Entity)` links the control to its spawned text display.
- `UiXmlControlName` and `UiXmlControlScope` expose optional form name and
  nearest form/document scope.
- `UiXmlDisabled(pub bool)` remains the enabled/disabled source of truth.
- `UiXmlTextChanged` is emitted only by crate-owned text input handling.

XML `value`, `name`, and `disabled` attributes seed these components during
spawn. After spawn, `UiXmlElement.attributes` remains structural metadata and
does not drive runtime text behavior.

## Input Semantics

Only the entity stored in `UiXmlFocus.entity` can receive text input.

The runtime appends non-control `ReceivedCharacter` values to `UiXmlTextValue`.
Pressed `KeyCode::Back` removes the last Unicode scalar value when the current
value is non-empty. Each value mutation emits one `UiXmlTextChanged` event with
the entity, scope, optional name, previous value, and new value.

Clicking a non-disabled text input through Bevy `Interaction::Pressed` sets
`UiXmlFocus.entity` to that input.

Disabled text inputs ignore clicks and keyboard input. Programmatic mutation of
`UiXmlTextValue` does not emit events; external code that mutates values
directly owns its own notification path.

## Deferred

- Cursor position and text selection.
- Delete, arrow keys, clipboard, composition/IME, and platform editing
  shortcuts.
- Placeholder rendering semantics.
- Validation.
- Reset behavior.
- Submit behavior.
- Full form serialization.

## Consequences

This gives Bevy users a small executable text input contract without importing
browser form semantics wholesale. The data path stays component-driven, and the
deferred editing features can be added later without changing the initial value
or event ownership model.

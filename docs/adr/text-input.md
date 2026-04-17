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
- `UiXmlTextPlaceholder` stores display-only placeholder text and text-style
  presentation for the empty-value state.
- `UiXmlControlName` and `UiXmlControlScope` expose optional form name and
  nearest form/document scope.
- `UiXmlDisabled(pub bool)` remains the enabled/disabled source of truth.
- `UiXmlTextChanged` is emitted only by crate-owned text input handling.

XML `value`, `name`, and `disabled` attributes seed these components during
spawn. After spawn, `UiXmlElement.attributes` remains structural metadata and
does not drive runtime text behavior.

## Input Semantics

Only the entity stored in `UiXmlFocus.entity` can receive text input.

The runtime inserts non-control `ReceivedCharacter` values at
`UiXmlTextCursor`. `KeyCode::Back` removes the scalar before the cursor,
`Delete` removes the scalar at the cursor, and `Left`/`Right`/`Home`/`End` move
the cursor. Each value mutation emits one `UiXmlTextChanged` event with the
entity, scope, optional name, previous value, and new value.

Clicking a non-disabled text input through Bevy `Interaction::Pressed` sets
`UiXmlFocus.entity` to that input.

Disabled text inputs ignore clicks and keyboard input. Programmatic mutation of
`UiXmlTextValue` does not emit events; external code that mutates values
directly owns its own notification path.

## Placeholder Semantics

XML `placeholder` seeds fallback display text. The placeholder is shown only
when `UiXmlTextValue` is empty. It is never copied into `UiXmlTextValue` and
does not emit `UiXmlTextChanged` when it appears, disappears, or reappears.

Placeholder styling is JSON-native through a nested `placeholder` block on the
input style object, for example:

```json
{
  "#email": {
    "color": "white",
    "fontSize": 16,
    "placeholder": { "color": "gray", "fontSize": 12 }
  }
}
```

`::placeholder` selector syntax is supported as a bounded alias for the
placeholder style block. The implementation maps placeholder text color and font
size onto the existing child `TextBundle`; layout/render-heavy placeholder
properties are not separate pseudo-elements.

## Deferred

- OS clipboard integration beyond the crate-owned in-memory clipboard resource.
- Platform editing shortcuts beyond explicit request events and Bevy key events.
- Browser validation UI.

## Consequences

This gives Bevy users a small executable text input contract without importing
browser form semantics wholesale. The data path stays component-driven, and the
deferred editing features can be added later without changing the initial value
or event ownership model.

## Selection, Clipboard, And IME

`UiXmlTextSelection` stores an anchor/focus character range. Selection can be
set explicitly with `UiXmlTextSelectAllRequested`, and character/IME/clipboard
insertions replace the selected range. `UiXmlClipboard` is an in-memory Bevy
resource used by `UiXmlClipboardCopyRequested`, `UiXmlClipboardCutRequested`,
and `UiXmlClipboardPasteRequested`; it intentionally does not access the OS
clipboard. `UiXmlImePreedit` records Bevy `Ime::Preedit`, while `Ime::Commit`
inserts committed text through the same component-owned text mutation path.

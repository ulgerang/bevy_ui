# ADR: Widget And Form Metadata Contract

Status: Accepted for Stage 7.

## Decision

The parser preserves original tags and attributes while adding type-aware identity for bounded controls:

- `<input type="checkbox">` and `<checkbox>` have widget type `checkbox`.
- `<input type="radio">` and `<radio>` have widget type `radio`.
- `<input type="range">` has widget type `range`; other unsupported input types remain metadata-only `input` nodes.
- `<form>` marks a control scope for descendants.

Executable behavior is represented by typed components such as `UiXmlControlKind`, `UiXmlChecked`, `UiXmlControlValue`, `UiXmlControlName`, and `UiXmlControlScope`. Raw `UiXmlElement.attributes` remains structural metadata.

## Drivers

- Selectors still need stable tag and attribute metadata.
- Runtime checkbox/radio behavior needs typed Bevy components, not stringly metadata.
- Unknown tags and attributes must remain preserved.

## Consequences

- Existing button/text/image behavior remains unchanged.
- Textarea/select/range/progress/meter/scroll now have bounded game-widget runtime components.
- Unsupported form controls can still be styled and inspected as metadata.
- Checkbox/radio can be executable without making every browser form control executable.

## Deferred

- Browser-compatible validation UI.
- OS-native select/dropdown behavior.
- Virtualized scroll lists.
- Advanced textarea line navigation and visual selection rendering.


## Game Widget Runtime Pack

`UiXmlTextArea`, `UiXmlSelect`, `UiXmlOption`, `UiXmlRange`, `UiXmlProgress`,
`UiXmlMeter`, and `UiXmlScrollContainer` provide bounded game UI behavior. These
widgets reuse existing focus/navigation, selected/open pseudo-states, form
serialization, and fill-percent metadata instead of implementing browser-complete
controls.

# ADR: Widget And Form Metadata Contract

Status: Accepted for Stage 7.

## Decision

The parser preserves original tags and attributes while adding type-aware identity for bounded controls:

- `<input type="checkbox">` and `<checkbox>` have widget type `checkbox`.
- `<input type="radio">` and `<radio>` have widget type `radio`.
- Other `<input>` elements remain metadata-only `input` nodes.
- `<form>` marks a control scope for descendants.

Executable behavior is represented by typed components such as `UiXmlControlKind`, `UiXmlChecked`, `UiXmlControlValue`, `UiXmlControlName`, and `UiXmlControlScope`. Raw `UiXmlElement.attributes` remains structural metadata.

## Drivers

- Selectors still need stable tag and attribute metadata.
- Runtime checkbox/radio behavior needs typed Bevy components, not stringly metadata.
- Unknown tags and attributes must remain preserved.

## Consequences

- Existing button/text/image behavior remains unchanged.
- Non-checkbox/radio form controls can be styled and inspected as metadata.
- Checkbox/radio can be executable without making all form controls executable.

## Deferred

- Text input editing.
- Textarea/select behavior.
- Form submit/reset/serialization.
- Browser-compatible validation.

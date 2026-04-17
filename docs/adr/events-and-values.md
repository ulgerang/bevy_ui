# ADR: Checkbox And Radio Events And Values

Status: Accepted for Stage 6.

## Decision

Checkbox and radio controls use Bevy components as runtime sources of truth:

- `UiXmlChecked(pub bool)` owns checked state after spawn.
- `UiXmlControlValue(pub String)` owns bounded-control value metadata after spawn.
- `UiXmlControlName(pub String)` is present only for non-empty normalized names.
- `UiXmlControlScope(pub Entity)` scopes radio groups to the nearest `<form>` entity or the document root entity.
- `UiXmlDisabled(pub bool)` remains the only runtime enabled/disabled source of truth.
- `UiXmlControlChanged` is emitted only by the crate interaction system.

XML attributes seed these components at spawn time. After spawn, `UiXmlElement.attributes` remains structural metadata and does not drive runtime checked, value, name, scope, or disabled behavior.

## Event Semantics

`UiXmlControlChanged` contains the entity, control kind, scope, optional name, value, checked state, and previous checked state.

Programmatic `UiXmlChecked` mutation does not emit events. External code that mutates `UiXmlChecked` directly owns its own notification path.

Checkbox interaction toggles checked state and emits one event.

Radio interaction:

- Selecting an unchecked radio checks it.
- If the radio has a non-empty name, checked peers in the same `(scope, name)` group are cleared.
- One event is emitted for each entity whose checked state changes.
- Selecting an already checked radio is a no-op and emits no event.
- Disabled interactions are no-ops and emit no event.

Missing checkbox/radio `value` defaults to `"on"`.

## Radio Initialization

When multiple radios in the same non-empty `(UiXmlControlScope, UiXmlControlName)` group are initially checked, the last radio in document order remains checked and earlier checked peers are cleared. Initialization emits no `UiXmlControlChanged` events.

Radios without a non-empty name are independent and are not grouped with each other.

## Deferred

- Text input is covered by `docs/adr/text-input.md`.
- Validation.
- Reset behavior.
- Submit behavior.
- Full form serialization.
- General callback binding.

## Consequences

The first interactive form slice is limited to checkbox/radio behavior. Public names avoid claiming complete form semantics while still giving Bevy users explicit component/event contracts.

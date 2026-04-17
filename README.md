# bevy_ui_xml

HTML/XML and CSS-like declarative UI for Bevy.

This project is a Rust/Bevy take on the structure used by
[`ebitenui-xml`](https://github.com/ulgerang/ebitenui-xml): UI hierarchy is
defined in a familiar XML/HTML-like document, while visual and flex layout
styles live in a CSS-like JSON stylesheet.

## Status

Early MVP. The parser and style cascade are in place, and the runtime builder
can spawn basic Bevy UI nodes.

Supported elements:

- `<ui>`, `<panel>`, `<div>`, `<container>` as containers
- `<text>`, `<label>`, `<span>`, `<p>` as text
- `<button>`, `<btn>` as buttons
- `<image>`, `<img>` as UI images
- `<form>` as a control scope
- `<input type="checkbox">`, `<checkbox>`, `<input type="radio">`, and
  `<radio>` as bounded interactive controls
- `<input type="text">` and `<input>` as bounded text controls
- other `<input>`, `<textarea>`, `<select>`, and `<option>`-style form nodes as
  structural metadata only

Supported selectors:

- tag, class, and ID selectors: `button`, `.danger`, `#save`
- compound selectors: `button.primary`, `button#save`
- descendant and child selectors: `.menu button`, `.menu > button`
- attribute selectors: `[disabled]`, `[type=submit]`
- static disabled pseudo selector: `button:disabled` in computed styles
- runtime `:hover`, `:active`, `:focus`, and `:disabled` styling for spawned UI
  when `UiXmlPlugin` is installed
- nested state styles: `hover`, `active`, `focus`, and `disabled`

Bounded controls:

- XML `checked`, `value`, `name`, and `disabled` seed Bevy components during
  spawn; after spawn, components are the source of truth.
- `UiXmlChecked` owns checkbox/radio checked state.
- `UiXmlControlValue`, `UiXmlControlName`, and `UiXmlControlScope` expose the
  bounded-control value, group name, and form/document scope.
- `UiXmlControlKind` identifies executable checkbox/radio controls.
- `UiXmlControlChanged` is emitted only for crate-handled checkbox/radio
  interactions.
- Radio groups are scoped by nearest `<form>` or the document root.
- Radios without a non-empty `name` are independent.
- Multiple initially checked radios in the same group normalize to the last
  radio in document order without emitting events.

Text inputs:

- XML `value`, `name`, and `disabled` seed Bevy components during spawn; after
  spawn, components are the source of truth.
- `UiXmlTextValue` owns the mutable text value.
- `UiXmlTextInput` identifies executable text controls.
- `UiXmlTextChanged` is emitted only for crate-handled text edits.
- Only the entity in `UiXmlFocus.entity` receives keyboard text.
- Clicking a non-disabled text input sets `UiXmlFocus.entity`.
- `ReceivedCharacter` appends non-control characters; `KeyCode::Back` removes
  the last character.

```rust
use bevy::prelude::*;
use bevy_ui_xml::{UiXmlBuilder, UiXmlControlChanged, UiXmlPlugin};

const LAYOUT: &str = r#"
<ui id="root">
    <form id="settings">
        <checkbox id="sound" name="sound" value="enabled" checked="true" />
        <input id="small" type="radio" name="size" value="small" checked="true" />
        <input id="large" type="radio" name="size" value="large" />
    </form>
</ui>
"#;

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ui = UiXmlBuilder::from_strings(LAYOUT, r#"{}"#).unwrap();
    ui.spawn(&mut commands, &asset_server);
}

fn read_control_changes(mut changes: EventReader<UiXmlControlChanged>) {
    for change in changes.read() {
        info!(
            "control {:?} value={} checked={}",
            change.name, change.value, change.checked
        );
    }
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, UiXmlPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, read_control_changes)
        .run();
}
```

Supported style groups include sizing, padding, margin, border width/color,
outline width/color/offset, absolute/relative positioning, overflow clipping,
aspect ratio, flex direction, wrap, align/justify, gaps, flex grow/shrink/basis,
background, text color, font size, text alignment/wrap, opacity, visibility,
z-index, and display.

Unsupported JSON properties are recorded in `StyleSheet::diagnostics` instead
of being silently ignored. Unsupported visual effects such as `boxShadow`,
`borderRadius`, `filter`, and `backdropFilter` are also preserved on spawned
entities as spawn-time `UiXmlUnsupportedEffects` metadata for a future custom
material renderer. Runtime state restyling does not update these unsupported
effect metadata snapshots.

Runtime state styling is intentionally Bevy-scoped. XML `disabled` seeds a
mutable `UiXmlDisabled` component during spawn; after that, the component is the
source of truth. Runtime `:focus` styling uses the crate-owned `UiXmlFocus`
resource; set `UiXmlFocus.entity` to the focused entity. Disabled entities do
not become effectively focused. Text input supports a bounded focused-editing
MVP. Cursor movement, text selection, IME/composition, placeholder rendering,
validation, reset, submit behavior, and full form serialization are not
implemented.

## Example

```rust
use bevy::prelude::*;
use bevy_ui_xml::{UiXmlBuilder, UiXmlPlugin};

const LAYOUT: &str = r#"
<ui id="root" width="100%" height="100%">
    <panel id="menu" class="card" direction="column">
        <text id="title">Bevy UI XML</text>
        <btn class="primary">Start</btn>
        <button class="danger">Quit</button>
    </panel>
</ui>
"#;

const STYLES: &str = r##"
{
    "styles": {
        "#root": {
            "direction": "column",
            "alignItems": "center",
            "justifyContent": "center",
            "background": "#111827"
        },
        ".card": {
            "width": 320,
            "padding": {"all": 20},
            "border-width": {"all": 2},
            "border-color": "dodgerblue",
            "outline-width": 1,
            "outline-color": "gold",
            "gap": 12,
            "background": "#1f2937"
        },
        ".card > button.primary": {
            "height": 44,
            "background": "royalblue",
            "color": "white",
            "hover": {
                "background": "dodgerblue"
            },
            "active": {
                "background": "darkred"
            }
        },
        ".danger": {
            "background": "crimson"
        },
        "#title": {
            "fontSize": 24,
            "color": "white"
        }
    }
}
"##;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, UiXmlPlugin))
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let ui = UiXmlBuilder::from_strings(LAYOUT, STYLES).unwrap();
    ui.spawn(&mut commands, &asset_server);
}
```

## Design Notes

- XML is parsed with `roxmltree`; current syntax is intentionally XML-valid
  HTML-like markup rather than a browser-compatible HTML parser.
- Styles are JSON for parity with the reference project. A real `.css` parser
  can be layered later without changing the Bevy spawn path.
- Runtime state and behavior boundaries are tracked in `docs/adr/`.
- Text uses Bevy's embedded default ASCII font by default. Call
  `with_default_font("path/in/assets.ttf")` when a project needs a specific
  font or non-ASCII glyph coverage.
- Image `src` and default font paths are handed to Bevy `AssetServer`; this
  crate does not provide a custom `AssetLoader` or hot reload contract.
- Bevy dependency is pinned to `0.12.1` because this workspace currently uses
  Rust `1.75.0`.

# bevy_ui_xml

HTML/XML and CSS-like declarative UI for Bevy.

This project is a Rust/Bevy take on the structure used by
[`ebitenui-xml`](https://github.com/ulgerang/ebitenui-xml): UI hierarchy is
defined in a familiar XML/HTML-like document, while visual and flex layout
styles live in a JSON stylesheet with tag, class, and ID selectors.

## Status

Early MVP. The parser and style cascade are in place, and the runtime builder
can spawn basic Bevy UI nodes.

Supported elements:

- `<ui>`, `<panel>`, `<div>`, `<container>` as containers
- `<text>`, `<label>`, `<span>`, `<p>` as text
- `<button>`, `<btn>` as buttons
- `<image>`, `<img>` as UI images

Supported selector precedence:

1. tag selector, for example `button`
2. class selector, for example `.danger`
3. ID selector, for example `#save`
4. inline XML attributes for `width`, `height`, and `direction`

## Example

```rust
use bevy::prelude::*;
use bevy_ui_xml::UiXmlBuilder;

const LAYOUT: &str = r#"
<ui id="root" width="100%" height="100%">
    <panel id="menu" class="card" direction="column">
        <text id="title">Bevy UI XML</text>
        <button class="primary">Start</button>
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
            "gap": 12,
            "background": "#1f2937"
        },
        "button": {
            "height": 44,
            "background": "royalblue",
            "color": "white"
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
- Bevy dependency is pinned to `0.12.1` because this workspace currently uses
  Rust `1.75.0`.


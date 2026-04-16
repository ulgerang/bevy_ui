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

Supported selectors:

- tag, class, and ID selectors: `button`, `.danger`, `#save`
- compound selectors: `button.primary`, `button#save`
- descendant and child selectors: `.menu button`, `.menu > button`
- attribute selectors: `[disabled]`, `[type=submit]`
- static disabled pseudo selector: `button:disabled`
- nested button state styles: `hover`, `active`, and `disabled` when
  `UiXmlPlugin` is installed

Supported style groups include sizing, padding, margin, border width/color,
outline width/color/offset, absolute/relative positioning, overflow clipping,
aspect ratio, flex direction, wrap, align/justify, gaps, flex grow/shrink/basis,
background, text color, font size, text alignment/wrap, opacity, visibility,
z-index, and display.

Unsupported JSON properties are recorded in `StyleSheet::diagnostics` instead
of being silently ignored. Unsupported visual effects such as `boxShadow`,
`borderRadius`, `filter`, and `backdropFilter` are also preserved on spawned
entities as `UiXmlUnsupportedEffects` for a future custom material renderer.

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
- Text uses Bevy's embedded default ASCII font by default. Call
  `with_default_font("path/in/assets.ttf")` when a project needs a specific
  font or non-ASCII glyph coverage.
- Bevy dependency is pinned to `0.12.1` because this workspace currently uses
  Rust `1.75.0`.

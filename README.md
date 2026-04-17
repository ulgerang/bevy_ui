# bevy_ui_xml

HTML/XML and CSS-like declarative UI for Bevy.

This project is a Rust/Bevy take on the structure used by
[`ebitenui-xml`](https://github.com/ulgerang/ebitenui-xml): UI hierarchy is
defined in a familiar XML/HTML-like document, while visual and flex layout
styles live in a CSS-like JSON stylesheet or a bounded native CSS rule-block subset.

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
- selector groups: `button, input`; each group member keeps its own
  specificity and shares the original JSON source order
- descendant and child selectors: `.menu button`, `.menu > button`
- attribute selectors: `[disabled]`, `[type=submit]`, `[class~=primary]`,
  `[lang|=en]`, `[href^=https]`, `[src$=.png]`, and `[data-id*=card]`
- static disabled pseudo selector: `button:disabled` in computed styles
- terminal runtime `:hover`, `:active`, `:focus`, `:focus-visible`,
  `:focus-within`, `:checked`, and `:disabled` styling for spawned UI when
  `UiXmlPlugin` is installed
- nested state styles: `hover`, `active`, `focus`, `focusVisible`,
  `focusWithin`, `checked`, and `disabled` (kebab-case aliases are accepted in
  JSON and normalized internally)

Unsupported selector syntax is reported through `StyleSheet::diagnostics`.
`::placeholder` is supported as a placeholder-style pseudo-element. Runtime
selector-chain invalidation is supported for ancestor state forms that can be
computed from retained entity context, such as `.form:focus-within .field` and
`.tabs:checked > .panel`; full CSSOM-style invalidation remains out of scope.

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


Game navigation:

- Interactive controls are focusable by default: buttons, checkboxes, radios, and text inputs.
- Non-control nodes can opt in with `focusable="true"` or `tabindex`/`tabIndex`.
- `nav-up`, `nav-down`, `nav-left`, and `nav-right` (or `focus-*` aliases) can target element IDs for explicit directional navigation.
- Keyboard navigation supports `Tab`, arrow keys, `Enter` activation, and `Escape` back intent.
- Gamepad navigation supports D-pad/left-stick direction, South activation, and East back intent through Bevy gamepad events.
- Navigation emits `UiXmlFocusChanged`, `UiXmlActivateRequested`, and `UiXmlBackRequested`; it updates `UiXmlFocus` and preserves `:focus-visible` semantics.
- Disabled, hidden, and `display: none` focusables are skipped.

Text inputs:

- XML `value`, `name`, and `disabled` seed Bevy components during spawn; after
  spawn, components are the source of truth.
- `UiXmlTextValue` owns the mutable text value.
- `UiXmlTextInput` identifies executable text controls.
- `UiXmlTextChanged` is emitted only for crate-handled text edits.
- Only the entity in `UiXmlFocus.entity` receives keyboard text.
- Clicking a non-disabled text input sets `UiXmlFocus.entity`.
- `ReceivedCharacter` inserts non-control characters at `UiXmlTextCursor`;
  `Back`, `Delete`, `Left`, `Right`, `Home`, and `End` provide bounded cursor
  editing.
- XML `placeholder` is display-only fallback text. It appears only while
  `UiXmlTextValue` is empty, never mutates `UiXmlTextValue`, and never emits
  `UiXmlTextChanged`.
- Placeholder style uses either a JSON-native nested `placeholder` block or a
  bounded CSS `input::placeholder` rule.

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


Game CSS tokens and transitions:

- Custom properties such as `--accent` can be declared in JSON/native CSS style maps.
- `var(--token)` and `var(--token, fallback)` resolve through explicit theme tokens, root tokens, rule-local tokens, then fallback.
- Runtime pseudo-states include `:selected`, `:open`, `:valid`, and `:invalid` for game widgets and validation styling.
- `transition` supports a bounded subset for background/opacity-oriented polish; unsupported transition properties are diagnostics, not browser animation parity.

Supported style groups include sizing, padding, margin, border width/color,
border side widths, outline width/color/offset, absolute/relative positioning,
`inset`, overflow clipping, aspect ratio, flex direction, wrap, align/justify,
gaps, flex grow/shrink/basis, a bounded `flex` shorthand (`none`, `auto`,
numeric grow, and grow/shrink/basis forms), background, text color, font size,
font family/weight/style metadata, text alignment/wrap, opacity, visibility,
z-index, display, JSON-native placeholder text style, and bounded native CSS rule blocks.

Color parsing accepts deterministic Bevy-mappable subsets: hex colors including
short `#rgb`/`#rgba`, `rgb()`/`rgba()` comma and space/slash forms, selected
named colors, and first-stop gradient fallback. Length parsing accepts numbers,
signed/decimal `px`, percentages, and `auto`; unsupported CSS units such as
`em`, `rem`, `vh`, `vw`, and `calc()` fall back to Bevy `Val::Auto`.

Unsupported JSON properties are recorded in `StyleSheet::diagnostics` instead
of being silently ignored. Unsupported visual effects such as `boxShadow`,
`borderRadius`, `filter`, and `backdropFilter` are also preserved on spawned
entities as spawn-time `UiXmlUnsupportedEffects` metadata for a future custom
material renderer. Runtime state restyling does not update these unsupported
effect metadata snapshots. Side-specific border colors are captured as
`UiXmlBorderColors` runtime metadata but are not rendered through Bevy UI
0.13's single `BorderColor`.

Runtime state styling is intentionally Bevy-scoped. XML `disabled` seeds a
mutable `UiXmlDisabled` component during spawn; after that, the component is the
source of truth. Runtime `:focus` styling uses the crate-owned `UiXmlFocus`
resource; set `UiXmlFocus.entity` to the focused entity. Disabled entities do
not become effectively focused. `UiXmlInputModality` separates pointer focus
from keyboard-visible focus for `:focus-visible`. Text input supports bounded
cursor editing plus display-only placeholders. Forms support component-owned
serialization, reset, submit, required-field validation events, validation-state
components, and navigation-intent events. Text inputs support component-owned
selection, in-memory clipboard request events, and Bevy IME preedit/commit
events. `UiXmlRenderMaterialSpec` plus `UiXmlEffectMaterialPlugin` provide an opt-in
Bevy `UiMaterial` shader path for effect-capable nodes. Full CSSOM, OS
clipboard integration, and browser validation UI/navigation remain integration
work outside this crate core.

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


Asset-backed UI loading:

- `UiXmlAssetPlugin` registers XML and CSS/style asset loaders without changing the string-first API.
- `UiXmlLayoutAsset` stores parsed XML documents and source-path diagnostics.
- `UiXmlStyleAsset` stores parsed stylesheets and path-aware diagnostics.
- `UiXmlAssetDocument` can spawn from layout/style handles once both assets are loaded.
- Matching asset reload events rebuild asset-backed child UI and bump style generation.

## Design Notes

- XML is parsed with `roxmltree`; current syntax is intentionally XML-valid
  HTML-like markup rather than a browser-compatible HTML parser.
- Styles are JSON for parity with the reference project. A real `.css` parser
  can be layered later without changing the Bevy spawn path.
- Runtime state and behavior boundaries are tracked in `docs/adr/`.
- Text uses Bevy's embedded default ASCII font by default. Call
  `with_default_font("path/in/assets.ttf")` when a project needs a specific
  font or non-ASCII glyph coverage.
- Image `src` and default font paths are handed to Bevy `AssetServer`. For
  XML/style document loading, add `UiXmlAssetPlugin` alongside Bevy
  `AssetPlugin` and use `UiXmlLayoutAsset`, `UiXmlStyleAsset`, and
  `UiXmlAssetDocument`/`spawn_asset_document`. Style asset events and
  `UiXmlThemeTokens` changes bump `UiXmlStyleRuntime.generation`; asset-backed
  roots rebuild their child UI on matching layout/style reloads.
- Bevy dependency is pinned to `0.13.2` for the current compatibility target.

### Optional effect material renderer

Projects using Bevy render/default plugins can opt into the included shader
material path with `UiXmlEffectMaterialPlugin`. The core `UiXmlPlugin` remains
headless-test friendly and only creates material handles when an
`Assets<UiXmlEffectMaterial>` resource is present. The shader is intentionally a
bounded first pass: it tints effect nodes, applies approximate rounded alpha,
and darkens shadowed edges. It is not a browser renderer for filters, backdrop
filters, or layout-affecting shadows.

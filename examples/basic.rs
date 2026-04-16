use bevy::prelude::*;
use bevy_ui_xml::UiXmlBuilder;

const LAYOUT: &str = r#"
<ui id="root" width="100%" height="100%">
    <panel id="menu" class="card" direction="column">
        <text id="title">Bevy UI XML</text>
        <text class="muted">HTML-like structure with CSS-like JSON styles.</text>
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
            "width": 360,
            "padding": {"all": 20},
            "gap": 12,
            "background": "#1f2937"
        },
        "button": {
            "height": 44,
            "background": "royalblue",
            "color": "white",
            "fontSize": 18
        },
        ".danger": {
            "background": "crimson"
        },
        "#title": {
            "fontSize": 28,
            "color": "white"
        },
        ".muted": {
            "fontSize": 14,
            "color": "gray"
        }
    }
}
"##;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    let ui =
        UiXmlBuilder::from_strings(LAYOUT, STYLES).expect("example layout and styles should parse");

    ui.spawn(&mut commands, &asset_server);
}

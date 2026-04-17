//! Declarative Bevy UI from HTML-like XML and CSS-like JSON.
//!
//! The public surface intentionally mirrors the reference `ebitenui-xml`
//! project: load a layout string, load a style sheet string, then spawn a UI
//! tree into Bevy.

use thiserror::Error;

mod builder;
mod parser;
mod render_effects;
mod runtime;
mod selector;
mod style;

pub use builder::{spawn_document, spawn_document_with_embedded_font, UiXmlBuilder};
pub use parser::{parse_layout, ElementNode, UiDocument};
pub use render_effects::{UiXmlUnsupportedEffects, UnsupportedEffect};
pub use runtime::{
    UiXmlChecked, UiXmlControlChanged, UiXmlControlKind, UiXmlControlName, UiXmlControlScope,
    UiXmlControlValue, UiXmlDisabled, UiXmlDocumentOrder, UiXmlElement, UiXmlFocus, UiXmlForm,
    UiXmlPlugin, UiXmlRuntimeState, UiXmlSelectorContext, UiXmlSelectorContextCache,
    UiXmlSelectorSnapshot, UiXmlStateStyles, UiXmlStyleRuntime, UiXmlStyleSource,
};
pub use style::{
    AlignSelfValue, AlignValue, DisplayValue, EdgeSizes, FlexDirectionValue, FlexWrapValue,
    JustifyValue, Length, OutlineStyle, OverflowValue, PositionValue, StyleDiagnostic, StyleSheet,
    TextAlignValue, TextWrapValue, UiStyle, VisibilityValue,
};

#[derive(Debug, Error)]
pub enum BevyUiXmlError {
    #[error("failed to parse XML layout: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("failed to parse JSON styles: {0}")]
    Json(#[from] serde_json::Error),
    #[error("layout is empty")]
    EmptyLayout,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_effects::{outline_from_style, unsupported_effects_from_style};
    use crate::selector::{Combinator, PseudoClass, Selector};
    use crate::style::parse_color;
    use bevy::prelude::*;

    #[test]
    fn parses_html_like_xml() {
        let doc = parse_layout(
            r#"<ui id="root"><div class="panel primary"><span>Hello</span></div></ui>"#,
        )
        .unwrap();

        assert_eq!(doc.root.tag, "ui");
        assert_eq!(doc.root.id.as_deref(), Some("root"));
        assert_eq!(doc.root.children[0].tag, "div");
        assert_eq!(doc.root.children[0].classes, ["panel", "primary"]);
        assert_eq!(doc.root.children[0].children[0].text, "Hello");
    }

    #[test]
    fn cascades_tag_class_and_id_styles() {
        let doc = parse_layout(r#"<button id="save" class="danger">Save</button>"#).unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "button": {"width": 100, "background": "#000000"},
                    ".danger": {"background": "#ff0000", "height": 30},
                    "#save": {"width": 180}
                }
            }"##,
        )
        .unwrap();

        let style = sheet.computed_style(&doc.root);
        assert_eq!(style.width, Some(Length::Px(180.0)));
        assert_eq!(style.height, Some(Length::Px(30.0)));
        assert_eq!(style.background.as_deref(), Some("#ff0000"));
    }

    #[test]
    fn supports_rootless_style_map_like_the_reference_project() {
        let sheet = StyleSheet::parse(r##"{"#root": {"direction": "column"}}"##).unwrap();
        assert!(sheet.styles.contains_key("#root"));
    }

    #[test]
    fn aliases_receive_canonical_tag_styles() {
        let doc = parse_layout(
            r#"
            <ui>
                <div id="card">
                    <btn id="save">Save</btn>
                    <span id="caption">Ready</span>
                    <img id="avatar"/>
                </div>
            </ui>
            "#,
        )
        .unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "panel": {"gap": 8},
                    "button": {"height": 42},
                    "text": {"fontSize": 18},
                    "image": {"width": 64}
                }
            }"##,
        )
        .unwrap();

        let div_style = sheet.computed_style(&doc.root.children[0]);
        let btn_style = sheet.computed_style(&doc.root.children[0].children[0]);
        let span_style = sheet.computed_style(&doc.root.children[0].children[1]);
        let img_style = sheet.computed_style(&doc.root.children[0].children[2]);

        assert_eq!(div_style.gap, Some(8.0));
        assert_eq!(btn_style.height, Some(Length::Px(42.0)));
        assert_eq!(span_style.font_size, Some(18.0));
        assert_eq!(img_style.width, Some(Length::Px(64.0)));
    }

    #[test]
    fn original_tag_style_can_override_canonical_alias_style() {
        let doc = parse_layout(r#"<div id="card"></div>"#).unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "panel": {"width": 100},
                    "div": {"width": 200}
                }
            }"##,
        )
        .unwrap();

        let style = sheet.computed_style(&doc.root);
        assert_eq!(style.width, Some(Length::Px(200.0)));
    }

    #[test]
    fn parses_css_color_forms_used_by_reference_styles() {
        assert_eq!(
            parse_color(Some("rgba(76, 175, 80, 0.2)")).map(|color| color.as_rgba_u8()),
            Some([76, 175, 80, 51])
        );
        assert_eq!(
            parse_color(Some("#336699cc")).map(|color| color.as_rgba_u8()),
            Some([51, 102, 153, 204])
        );
        assert_eq!(
            parse_color(Some("linear-gradient(90deg, #16213e, #1a1a2e)"))
                .map(|color| color.as_rgba_u8()),
            Some([22, 33, 62, 255])
        );
        assert_eq!(
            parse_color(Some("tomato")).map(|color| color.as_rgba_u8()),
            Some([255, 99, 71, 255])
        );
        assert_eq!(
            parse_color(Some("royalblue")).map(|color| color.as_rgba_u8()),
            Some([65, 105, 225, 255])
        );
    }

    #[test]
    fn matches_compound_descendant_child_attribute_and_disabled_selectors() {
        let child_selector = Selector::parse(".menu > button").unwrap();
        assert_eq!(child_selector.parts[1].combinator, Some(Combinator::Child));

        let doc = parse_layout(
            r#"
            <ui>
                <div class="menu">
                    <button id="start" class="primary" disabled="true">Start</button>
                    <panel><button id="nested">Nested</button></panel>
                </div>
            </ui>
            "#,
        )
        .unwrap();
        let menu = &doc.root.children[0];
        let start = &menu.children[0];
        let nested = &menu.children[1].children[0];
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "button.primary": {"width": 100},
                    ".menu button": {"height": 40},
                    ".menu > button": {"fontSize": 18},
                    "[disabled=true]": {"opacity": 0.5},
                    "button:disabled": {"background": "gray"}
                }
            }"##,
        )
        .unwrap();

        let start_style = sheet.computed_style_for_path(&[&doc.root, menu, start]);
        let nested_style =
            sheet.computed_style_for_path(&[&doc.root, menu, &menu.children[1], nested]);

        assert_eq!(start_style.width, Some(Length::Px(100.0)));
        assert_eq!(start_style.height, Some(Length::Px(40.0)));
        assert_eq!(start_style.font_size, Some(18.0));
        assert_eq!(start_style.opacity, Some(0.5));
        assert_eq!(start_style.background.as_deref(), Some("gray"));
        assert_eq!(nested_style.height, Some(Length::Px(40.0)));
        assert_eq!(nested_style.font_size, None);
    }

    #[test]
    fn parses_more_css_like_box_and_position_properties() {
        let doc = parse_layout(r#"<div></div>"#).unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "panel": {
                        "position": "absolute",
                        "left": "10px",
                        "top": "5%",
                        "padding": [4, 8],
                        "border-width": {"all": 2},
                        "border-color": "tomato",
                        "overflow": "hidden",
                        "aspect-ratio": 1.5,
                        "flex-wrap": "wrap",
                        "align-self": "center",
                        "row-gap": 6,
                        "column-gap": "10px",
                        "flex-basis": "25%",
                        "outline-width": 3,
                        "outline-color": "gold",
                        "outline-offset": 1,
                        "z-index": 7,
                        "visibility": "hidden",
                        "text-align": "center",
                        "text-wrap": "no-wrap"
                    }
                }
            }"##,
        )
        .unwrap();

        let style = sheet.computed_style(&doc.root);
        assert_eq!(style.position, Some(PositionValue::Absolute));
        assert_eq!(style.left, Some(Length::Text("10px".to_string())));
        assert_eq!(style.top, Some(Length::Text("5%".to_string())));
        assert_eq!(
            style.border_width,
            Some(EdgeSizes::Sides {
                all: Some(Length::Px(2.0)),
                x: None,
                y: None,
                top: None,
                right: None,
                bottom: None,
                left: None,
            })
        );
        assert_eq!(style.border_color.as_deref(), Some("tomato"));
        assert_eq!(style.overflow, Some(OverflowValue::Hidden));
        assert_eq!(style.aspect_ratio, Some(1.5));
        assert_eq!(style.flex_wrap, Some(FlexWrapValue::Wrap));
        assert_eq!(style.align_self, Some(AlignSelfValue::Center));
        assert_eq!(style.row_gap, Some(Length::Px(6.0)));
        assert_eq!(style.column_gap, Some(Length::Text("10px".to_string())));
        assert_eq!(style.flex_basis, Some(Length::Text("25%".to_string())));
        assert_eq!(style.outline_width, Some(Length::Px(3.0)));
        assert_eq!(style.outline_color.as_deref(), Some("gold"));
        assert_eq!(style.outline_offset, Some(Length::Px(1.0)));
        assert_eq!(style.z_index, Some(7));
        assert_eq!(style.visibility, Some(VisibilityValue::Hidden));
        assert_eq!(style.text_align, Some(TextAlignValue::Center));
        assert_eq!(style.text_wrap, Some(TextWrapValue::NoWrap));
    }

    #[test]
    fn reports_unsupported_style_properties() {
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    ".card": {
                        "boxShadow": "0 4px 8px black",
                        "hover": {
                            "filter": "blur(2px)"
                        }
                    }
                }
            }"##,
        )
        .unwrap();

        assert_eq!(sheet.diagnostics.len(), 2);
        assert!(sheet.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic,
            StyleDiagnostic::UnsupportedEffect { property, .. } if property == "boxShadow"
        )));
        assert!(sheet.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic,
            StyleDiagnostic::UnsupportedEffect { property, .. } if property == "filter"
        )));
    }

    #[test]
    fn creates_native_outline_and_tracks_unsupported_effect_values() {
        let style = UiStyle {
            outline_width: Some(Length::Px(2.0)),
            outline_color: Some("tomato".to_string()),
            outline_offset: Some(Length::Px(3.0)),
            box_shadow: Some("0 4px 8px black".to_string()),
            border_radius: Some("8px".to_string()),
            ..Default::default()
        };

        let outline = outline_from_style(&style).unwrap();
        assert_eq!(outline.width, Val::Px(2.0));
        assert_eq!(outline.offset, Val::Px(3.0));
        assert_eq!(outline.color.as_rgba_u8(), [255, 99, 71, 255]);

        let effects = unsupported_effects_from_style(&style).unwrap();
        assert_eq!(effects.effects.len(), 2);
        assert!(effects
            .effects
            .contains(&UnsupportedEffect::BoxShadow("0 4px 8px black".to_string())));
        assert!(effects
            .effects
            .contains(&UnsupportedEffect::BorderRadius("8px".to_string())));
    }

    #[test]
    fn resolves_nested_button_state_styles() {
        let style = UiStyle {
            background: Some("royalblue".to_string()),
            hover: Some(Box::new(UiStyle {
                background: Some("dodgerblue".to_string()),
                ..Default::default()
            })),
            active: Some(Box::new(UiStyle {
                background: Some("darkred".to_string()),
                ..Default::default()
            })),
            disabled: Some(Box::new(UiStyle {
                opacity: Some(0.5),
                ..Default::default()
            })),
            ..Default::default()
        };
        let states = UiXmlStateStyles::from_style(&style);

        assert_eq!(
            states
                .resolve(Interaction::Hovered, false)
                .background
                .as_deref(),
            Some("dodgerblue")
        );
        assert_eq!(
            states
                .resolve(Interaction::Pressed, false)
                .background
                .as_deref(),
            Some("darkred")
        );
        assert_eq!(
            states.resolve(Interaction::Hovered, true).opacity,
            Some(0.5)
        );
    }

    #[test]
    fn characterizes_static_pseudo_classes_and_nested_state_blocks() {
        let doc = parse_layout(r#"<button id="save" disabled="true">Save</button>"#).unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "button": {"background": "black"},
                    "button:hover": {"background": "dodgerblue"},
                    "button:active": {"background": "darkred"},
                    "button:focus": {"outline-width": 3},
                    "button:disabled": {"opacity": 0.4},
                    "#save": {
                        "hover": {"background": "gold"},
                        "active": {"background": "tomato"},
                        "focus": {"outline-width": 2},
                        "disabled": {"background": "gray", "opacity": 0.5}
                    }
                }
            }"##,
        )
        .unwrap();

        let style = sheet.computed_style(&doc.root);
        assert_eq!(style.background.as_deref(), Some("black"));
        assert_eq!(style.opacity, Some(0.4));
        assert_eq!(style.outline_width, None);

        let states = UiXmlStateStyles::from_style(&style);
        assert_eq!(
            states
                .resolve(Interaction::Hovered, false)
                .background
                .as_deref(),
            Some("gold")
        );
        assert_eq!(
            states
                .resolve(Interaction::Pressed, false)
                .background
                .as_deref(),
            Some("tomato")
        );
        let disabled = states.resolve(Interaction::Hovered, true);
        assert_eq!(disabled.background.as_deref(), Some("gray"));
        assert_eq!(disabled.opacity, Some(0.5));
    }

    #[test]
    fn runtime_state_cascade_uses_bevy_state_instead_of_static_disabled_metadata() {
        let doc = parse_layout(r#"<button id="save" disabled="true">Save</button>"#).unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "button": {"background": "black"},
                    "button:hover": {"background": "dodgerblue"},
                    "button:active": {"background": "darkred"},
                    "button:disabled": {"opacity": 0.4},
                    "#save": {
                        "hover": {"background": "gold"},
                        "disabled": {"background": "gray", "opacity": 0.5}
                    }
                }
            }"##,
        )
        .unwrap();

        let base = sheet.runtime_base_style_for_path(&[&doc.root]);
        let hover = sheet.runtime_state_style_for_path(&[&doc.root], PseudoClass::Hover);
        let active = sheet.runtime_state_style_for_path(&[&doc.root], PseudoClass::Active);
        let focus = sheet.runtime_state_style_for_path(&[&doc.root], PseudoClass::Focus);
        let disabled = sheet.runtime_state_style_for_path(&[&doc.root], PseudoClass::Disabled);
        let states =
            UiXmlStateStyles::from_runtime_styles(&base, &hover, &active, &focus, &disabled);

        assert_eq!(base.background.as_deref(), Some("black"));
        assert_eq!(base.opacity, None);
        assert_eq!(
            states
                .resolve(Interaction::Hovered, false)
                .background
                .as_deref(),
            Some("gold")
        );
        assert_eq!(
            states
                .resolve(Interaction::Pressed, false)
                .background
                .as_deref(),
            Some("darkred")
        );
        let disabled = states.resolve(Interaction::Hovered, true);
        assert_eq!(disabled.background.as_deref(), Some("gray"));
        assert_eq!(disabled.opacity, Some(0.5));
    }

    #[test]
    fn runtime_system_restyles_when_interaction_or_disabled_changes() {
        let mut app = App::new();
        app.add_plugins(UiXmlPlugin);

        let base = UiStyle {
            background: Some("black".to_string()),
            ..Default::default()
        };
        let hover = UiStyle {
            background: Some("dodgerblue".to_string()),
            ..Default::default()
        };
        let active = UiStyle {
            background: Some("darkred".to_string()),
            ..Default::default()
        };
        let disabled = UiStyle {
            background: Some("gray".to_string()),
            opacity: Some(0.5),
            ..Default::default()
        };

        let entity = app
            .world
            .spawn((
                Interaction::None,
                UiXmlDisabled(false),
                UiXmlRuntimeState::default(),
                UiXmlStyleSource {
                    base,
                    hover,
                    active,
                    focus: UiStyle::default(),
                    disabled,
                },
                Style::default(),
                BackgroundColor(Color::NONE),
                BorderColor(Color::NONE),
            ))
            .id();

        app.update();
        app.world.entity_mut(entity).insert(Interaction::Hovered);
        app.update();
        assert_eq!(
            app.world
                .entity(entity)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .as_rgba_u8(),
            [30, 144, 255, 255]
        );

        app.world.entity_mut(entity).insert(UiXmlDisabled(true));
        app.update();
        assert_eq!(
            app.world
                .entity(entity)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .as_rgba_u8(),
            [128, 128, 128, 127]
        );

        app.world.entity_mut(entity).insert(UiXmlDisabled(false));
        app.update();
        assert_eq!(
            app.world
                .entity(entity)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .as_rgba_u8(),
            [30, 144, 255, 255]
        );
    }

    #[test]
    fn builder_spawn_preserves_nested_runtime_state_blocks() {
        const LAYOUT: &str = r#"<ui><button id="save">Save</button></ui>"#;
        const STYLES: &str = r##"{
            "styles": {
                "#save": {
                    "background": "black",
                    "hover": {"background": "dodgerblue"},
                    "disabled": {"background": "gray", "opacity": 0.5}
                }
            }
        }"##;

        let ui = UiXmlBuilder::from_strings(LAYOUT, STYLES).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            UiXmlPlugin,
        ));
        app.add_systems(
            Startup,
            move |mut commands: Commands<'_, '_>, asset_server: Res<'_, AssetServer>| {
                ui.spawn(&mut commands, &asset_server);
            },
        );

        app.update();
        let mut query = app.world.query::<(Entity, &UiXmlElement)>();
        let button = query
            .iter(&app.world)
            .find_map(|(entity, element)| (element.id.as_deref() == Some("save")).then_some(entity))
            .unwrap();

        app.world.entity_mut(button).insert(Interaction::Hovered);
        app.update();
        assert_eq!(
            app.world
                .entity(button)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .as_rgba_u8(),
            [30, 144, 255, 255]
        );

        app.world.entity_mut(button).insert(UiXmlDisabled(true));
        app.update();
        assert_eq!(
            app.world
                .entity(button)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .as_rgba_u8(),
            [128, 128, 128, 127]
        );
    }

    #[test]
    fn selector_context_cache_tracks_spawned_runtime_context() {
        let mut app = App::new();
        app.add_plugins(UiXmlPlugin);

        let doc = parse_layout(
            r#"<ui id="root"><div class="menu"><button id="save">Save</button></div></ui>"#,
        )
        .unwrap();
        let root = &doc.root;
        let menu = &root.children[0];
        let button = &menu.children[0];
        let root_entity = Entity::from_raw(1);
        let menu_context = UiXmlSelectorContext::from_node(menu, Some(root_entity), &[root]);

        let entity = app.world.spawn(menu_context.clone()).id();
        app.update();

        let cache = app.world.resource::<UiXmlSelectorContextCache>();
        let cached = cache.entities.get(&entity).unwrap();
        assert_eq!(cached.parent, Some(root_entity));
        assert_eq!(cached.ancestors.len(), 1);
        assert_eq!(cached.ancestors[0].id.as_deref(), Some("root"));
        assert_eq!(cached.id, menu_context.id);

        let button_context = UiXmlSelectorContext::from_node(button, Some(entity), &[root, menu]);
        app.world.entity_mut(entity).insert(button_context);
        app.update();
        let cache = app.world.resource::<UiXmlSelectorContextCache>();
        let cached = cache.entities.get(&entity).unwrap();
        assert_eq!(cached.parent, Some(entity));
        assert_eq!(cached.ancestors.len(), 2);
        assert_eq!(cached.id.as_deref(), Some("save"));
    }

    #[test]
    fn characterizes_source_order_specificity_inline_and_alias_precedence() {
        let doc = parse_layout(
            r#"
            <ui>
                <btn id="save" class="primary" width="320">Save</btn>
            </ui>
            "#,
        )
        .unwrap();
        let button = &doc.root.children[0];
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "button": {"width": 100, "height": 30, "background": "black"},
                    "button": {"height": 32},
                    ".primary": {"width": 180, "background": "royalblue"},
                    "button.primary": {"background": "tomato"},
                    "#save": {"height": 44},
                    "btn": {"fontSize": 20}
                }
            }"##,
        )
        .unwrap();

        let style = sheet.computed_style_for_path(&[&doc.root, button]);
        assert_eq!(style.width, Some(Length::Px(320.0)));
        assert_eq!(style.height, Some(Length::Px(44.0)));
        assert_eq!(style.background.as_deref(), Some("tomato"));
        assert_eq!(style.font_size, Some(20.0));
    }

    fn spawn_test_app(layout: &str, styles: &str) -> App {
        let ui = UiXmlBuilder::from_strings(layout, styles).unwrap();
        let mut app = App::new();
        app.add_plugins((
            MinimalPlugins,
            bevy::asset::AssetPlugin::default(),
            UiXmlPlugin,
        ));
        app.add_systems(
            Startup,
            move |mut commands: Commands<'_, '_>, asset_server: Res<'_, AssetServer>| {
                ui.spawn(&mut commands, &asset_server);
            },
        );
        app.update();
        app
    }

    fn entity_by_id(app: &mut App, id: &str) -> Entity {
        let mut query = app.world.query::<(Entity, &UiXmlElement)>();
        query
            .iter(&app.world)
            .find_map(|(entity, element)| (element.id.as_deref() == Some(id)).then_some(entity))
            .unwrap()
    }

    fn drain_control_events(app: &mut App) -> Vec<UiXmlControlChanged> {
        app.world
            .resource_mut::<Events<UiXmlControlChanged>>()
            .drain()
            .collect()
    }

    fn press_control(app: &mut App, entity: Entity) {
        app.world.entity_mut(entity).insert(Interaction::Pressed);
        app.update();
    }

    #[test]
    fn focus_runtime_uses_uixml_focus_resource_as_source_of_truth() {
        let doc = parse_layout(r#"<button id="save">Save</button>"#).unwrap();
        let sheet = StyleSheet::parse(
            r##"{
                "styles": {
                    "button": {"background": "black"},
                    "button:focus": {"outline-width": 3},
                    "#save": {"focus": {"outline-color": "gold"}}
                }
            }"##,
        )
        .unwrap();

        let base = sheet.runtime_base_style_for_path(&[&doc.root]);
        assert_eq!(base.background.as_deref(), Some("black"));
        assert_eq!(base.outline_width, None);
        assert_eq!(
            sheet
                .runtime_state_style_for_path(&[&doc.root], PseudoClass::Focus)
                .outline_width,
            Some(Length::Px(3.0))
        );

        let mut app = App::new();
        app.add_plugins(UiXmlPlugin);
        let entity = app
            .world
            .spawn((
                UiXmlRuntimeState {
                    focused: true,
                    ..Default::default()
                },
                UiXmlDisabled(false),
                UiXmlStyleSource::from_runtime_styles(
                    &base,
                    &UiStyle::default(),
                    &UiStyle::default(),
                    &sheet.runtime_state_style_for_path(&[&doc.root], PseudoClass::Focus),
                    &UiStyle::default(),
                ),
                Style::default(),
                BackgroundColor(Color::NONE),
                BorderColor(Color::NONE),
            ))
            .id();

        app.update();
        assert!(app.world.entity(entity).get::<Outline>().is_none());

        app.world.resource_mut::<UiXmlFocus>().entity = Some(entity);
        app.update();
        let outline = app.world.entity(entity).get::<Outline>().unwrap();
        assert_eq!(outline.width, Val::Px(3.0));
        assert_eq!(outline.color.as_rgba_u8(), [255, 215, 0, 255]);

        app.world.entity_mut(entity).insert(UiXmlDisabled(true));
        app.update();
        assert!(
            !app.world
                .entity(entity)
                .get::<UiXmlRuntimeState>()
                .unwrap()
                .focused
        );
        assert_eq!(
            app.world
                .entity(entity)
                .get::<Outline>()
                .unwrap()
                .color
                .as_rgba_u8(),
            [0, 0, 0, 0]
        );
    }

    #[test]
    fn parser_widgets_forms_distinguishes_control_identity_and_preserves_metadata() {
        let doc = parse_layout(
            r#"
            <form id="profile">
                <input id="agree" type="checkbox" name="terms" value="yes" checked="true" />
                <checkbox id="short" />
                <input id="small" type="radio" name="size" />
                <radio id="large" />
                <input id="email" type="text" placeholder="Email" />
            </form>
            "#,
        )
        .unwrap();

        let children = &doc.root.children;
        assert_eq!(doc.root.widget_type(), "form");
        assert_eq!(children[0].tag, "input");
        assert_eq!(children[0].widget_type(), "checkbox");
        assert_eq!(children[0].attr("type"), Some("checkbox"));
        assert_eq!(children[1].widget_type(), "checkbox");
        assert_eq!(children[2].tag, "input");
        assert_eq!(children[2].widget_type(), "radio");
        assert_eq!(children[3].widget_type(), "radio");
        assert_eq!(children[4].widget_type(), "input");
        assert_eq!(children[4].attr("placeholder"), Some("Email"));
    }

    #[test]
    fn builder_forms_assigns_scope_and_seeds_control_metadata() {
        let mut app = spawn_test_app(
            r#"
            <ui id="root">
                <form id="profile">
                    <input id="agree" type="checkbox" name="terms" value="yes" checked="true" disabled="true" />
                </form>
                <input id="outside" type="radio" name="size" />
            </ui>
            "#,
            r#"{}"#,
        );

        let root = entity_by_id(&mut app, "root");
        let form = entity_by_id(&mut app, "profile");
        let agree = entity_by_id(&mut app, "agree");
        let outside = entity_by_id(&mut app, "outside");

        let agree_entity = app.world.entity(agree);
        assert_eq!(
            agree_entity.get::<UiXmlControlKind>(),
            Some(&UiXmlControlKind::Checkbox)
        );
        assert_eq!(
            agree_entity.get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(
            agree_entity.get::<UiXmlDisabled>(),
            Some(&UiXmlDisabled(true))
        );
        assert_eq!(
            agree_entity
                .get::<UiXmlControlName>()
                .map(|name| name.0.as_str()),
            Some("terms")
        );
        assert_eq!(
            agree_entity
                .get::<UiXmlControlValue>()
                .map(|value| value.0.as_str()),
            Some("yes")
        );
        assert_eq!(
            agree_entity.get::<UiXmlControlScope>(),
            Some(&UiXmlControlScope(form))
        );
        assert!(app.world.entity(form).contains::<UiXmlForm>());
        assert_eq!(
            app.world.entity(outside).get::<UiXmlControlScope>(),
            Some(&UiXmlControlScope(root))
        );
    }

    #[test]
    fn checkbox_controls_events_toggle_and_respect_disabled() {
        let mut app = spawn_test_app(
            r#"
            <ui id="root">
                <input id="agree" type="checkbox" name="terms" value="yes" />
                <input id="blocked" type="checkbox" checked="true" disabled="true" />
            </ui>
            "#,
            r#"{}"#,
        );
        drain_control_events(&mut app);

        let agree = entity_by_id(&mut app, "agree");
        press_control(&mut app, agree);
        let events = drain_control_events(&mut app);

        assert_eq!(
            app.world.entity(agree).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, agree);
        assert_eq!(events[0].kind, UiXmlControlKind::Checkbox);
        assert_eq!(events[0].name.as_deref(), Some("terms"));
        assert_eq!(events[0].value, "yes");
        assert!(events[0].checked);
        assert!(!events[0].previous_checked);

        let blocked = entity_by_id(&mut app, "blocked");
        press_control(&mut app, blocked);
        let disabled_events = drain_control_events(&mut app);
        assert_eq!(
            app.world.entity(blocked).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert!(disabled_events.is_empty());
    }

    #[test]
    fn radio_controls_events_scope_grouping_and_initial_normalization() {
        let mut app = spawn_test_app(
            r#"
            <ui id="root">
                <form id="first">
                    <input id="small" type="radio" name="size" value="s" checked="true" />
                    <input id="large" type="radio" name="size" value="l" checked="true" />
                </form>
                <form id="second">
                    <input id="other" type="radio" name="size" value="o" checked="true" />
                </form>
                <input id="outside-a" type="radio" name="size" checked="true" />
                <input id="outside-b" type="radio" name="size" />
                <input id="unnamed-a" type="radio" checked="true" />
                <input id="unnamed-b" type="radio" />
            </ui>
            "#,
            r#"{}"#,
        );
        assert!(drain_control_events(&mut app).is_empty());

        let small = entity_by_id(&mut app, "small");
        let large = entity_by_id(&mut app, "large");
        let other = entity_by_id(&mut app, "other");
        assert_eq!(
            app.world.entity(small).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(false))
        );
        assert_eq!(
            app.world.entity(large).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(
            app.world.entity(other).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );

        press_control(&mut app, small);
        let form_events = drain_control_events(&mut app);
        assert_eq!(form_events.len(), 2);
        assert_eq!(
            app.world.entity(small).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(
            app.world.entity(large).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(false))
        );
        assert_eq!(
            app.world.entity(other).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert!(form_events
            .iter()
            .any(|event| event.entity == small && event.checked && !event.previous_checked));
        assert!(form_events
            .iter()
            .any(|event| event.entity == large && !event.checked && event.previous_checked));

        let outside_a = entity_by_id(&mut app, "outside-a");
        let outside_b = entity_by_id(&mut app, "outside-b");
        press_control(&mut app, outside_b);
        let outside_events = drain_control_events(&mut app);
        assert_eq!(outside_events.len(), 2);
        assert_eq!(
            app.world.entity(outside_a).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(false))
        );
        assert_eq!(
            app.world.entity(outside_b).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );

        let unnamed_a = entity_by_id(&mut app, "unnamed-a");
        let unnamed_b = entity_by_id(&mut app, "unnamed-b");
        press_control(&mut app, unnamed_b);
        let unnamed_events = drain_control_events(&mut app);
        assert_eq!(unnamed_events.len(), 1);
        assert_eq!(
            app.world.entity(unnamed_a).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(
            app.world.entity(unnamed_b).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
    }

    #[test]
    fn radio_controls_events_already_checked_peer_is_noop() {
        let mut app = spawn_test_app(
            r#"
            <ui id="root">
                <input id="small" type="radio" name="size" checked="true" />
                <input id="large" type="radio" name="size" />
            </ui>
            "#,
            r#"{}"#,
        );
        drain_control_events(&mut app);

        let small = entity_by_id(&mut app, "small");
        press_control(&mut app, small);
        let events = drain_control_events(&mut app);

        assert_eq!(
            app.world.entity(small).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert!(events.is_empty());
    }

    #[test]
    fn radio_controls_events_disabled_peer_is_not_cleared() {
        let mut app = spawn_test_app(
            r#"
            <ui id="root">
                <input id="locked" type="radio" name="size" checked="true" disabled="true" />
                <input id="large" type="radio" name="size" />
            </ui>
            "#,
            r#"{}"#,
        );
        drain_control_events(&mut app);

        let locked = entity_by_id(&mut app, "locked");
        let large = entity_by_id(&mut app, "large");
        press_control(&mut app, large);
        let events = drain_control_events(&mut app);

        assert_eq!(
            app.world.entity(locked).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(
            app.world.entity(large).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].entity, large);
        assert!(events[0].checked);

        app.update();
        assert_eq!(
            app.world.entity(locked).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert_eq!(
            app.world.entity(large).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert!(drain_control_events(&mut app).is_empty());
    }

    #[test]
    fn controls_events_programmatic_checked_mutation_emits_no_event() {
        let mut app = spawn_test_app(
            r#"<ui id="root"><input id="agree" type="checkbox" /></ui>"#,
            r#"{}"#,
        );
        drain_control_events(&mut app);

        let agree = entity_by_id(&mut app, "agree");
        app.world.entity_mut(agree).insert(UiXmlChecked(true));
        app.update();
        let events = drain_control_events(&mut app);

        assert_eq!(
            app.world.entity(agree).get::<UiXmlChecked>(),
            Some(&UiXmlChecked(true))
        );
        assert!(events.is_empty());
    }

    #[test]
    fn forms_controls_non_checkbox_radio_input_remains_metadata_only() {
        let mut app = spawn_test_app(
            r#"
            <ui id="root">
                <input id="email" type="text" name="email" value="hello@example.com" />
            </ui>
            "#,
            r#"{}"#,
        );
        drain_control_events(&mut app);

        let email = entity_by_id(&mut app, "email");
        let email_entity = app.world.entity(email);
        let element = email_entity.get::<UiXmlElement>().unwrap();

        assert_eq!(element.tag, "input");
        assert_eq!(element.widget_type, "input");
        assert_eq!(
            element.attributes.get("type").map(String::as_str),
            Some("text")
        );
        assert!(!email_entity.contains::<UiXmlControlKind>());
        assert!(!email_entity.contains::<UiXmlChecked>());
        assert!(!email_entity.contains::<UiXmlControlValue>());

        app.update();
        assert!(drain_control_events(&mut app).is_empty());
    }

    #[test]
    fn render_effects_unsupported_effects_runtime_metadata_remains_spawn_time_only() {
        let mut app = spawn_test_app(
            r#"<ui id="root"><button id="card">Card</button></ui>"#,
            r##"{
                "styles": {
                    "#card": {
                        "background": "black",
                        "boxShadow": "0 4px 8px black",
                        "hover": {
                            "background": "dodgerblue",
                            "boxShadow": "0 8px 16px black"
                        }
                    }
                }
            }"##,
        );

        let card = entity_by_id(&mut app, "card");
        let effects = app
            .world
            .entity(card)
            .get::<UiXmlUnsupportedEffects>()
            .unwrap()
            .clone();
        assert!(effects
            .effects
            .contains(&UnsupportedEffect::BoxShadow("0 4px 8px black".to_string())));

        app.world.entity_mut(card).insert(Interaction::Hovered);
        app.update();
        let after_hover = app
            .world
            .entity(card)
            .get::<UiXmlUnsupportedEffects>()
            .unwrap();
        assert_eq!(&effects, after_hover);
        assert_eq!(
            app.world
                .entity(card)
                .get::<BackgroundColor>()
                .unwrap()
                .0
                .as_rgba_u8(),
            [30, 144, 255, 255]
        );
    }

    #[test]
    fn asset_image_text_metadata_remains_bevy_asset_server_owned() {
        let doc = parse_layout(
            r#"
            <ui id="root">
                <text id="title" content="Hello" />
                <img id="avatar" src="textures/avatar.png" />
            </ui>
            "#,
        )
        .unwrap();

        let title = &doc.root.children[0];
        let avatar = &doc.root.children[1];
        assert_eq!(title.widget_type(), "text");
        assert_eq!(title.attr("content"), Some("Hello"));
        assert_eq!(avatar.widget_type(), "image");
        assert_eq!(avatar.attr("src"), Some("textures/avatar.png"));
    }
}

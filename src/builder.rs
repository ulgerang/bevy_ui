use crate::render_effects::{
    border_colors_from_style, outline_from_style, render_material_spec_from_style,
    unsupported_effects_from_style,
};
use crate::runtime::{
    apply_text_presentation, RuntimeStyleInputs, UiXmlChecked, UiXmlControlKind, UiXmlControlName,
    UiXmlControlScope, UiXmlControlValue, UiXmlDisabled, UiXmlDocumentOrder, UiXmlElement,
    UiXmlForm, UiXmlImePreedit, UiXmlInitialChecked, UiXmlInitialTextValue, UiXmlRequired,
    UiXmlRuntimeState, UiXmlSelectorContext, UiXmlStateStyles, UiXmlStyleSource, UiXmlTextCursor,
    UiXmlTextDisplay, UiXmlTextInput, UiXmlTextPlaceholder, UiXmlTextSelection, UiXmlTextValue,
    UiXmlValidationState,
};
use crate::selector::PseudoClass;
use crate::style::{style_color, to_bevy_style, StyleSheet, UiStyle, VisibilityValue};
use crate::{parse_layout, BevyUiXmlError, ElementNode, UiDocument};
use bevy::prelude::*;

#[derive(Debug, Clone)]
pub struct UiXmlBuilder {
    document: UiDocument,
    stylesheet: StyleSheet,
    default_font: Option<String>,
}

impl UiXmlBuilder {
    pub fn from_strings(layout: &str, styles: &str) -> Result<Self, BevyUiXmlError> {
        Ok(Self {
            document: parse_layout(layout)?,
            stylesheet: StyleSheet::parse(styles)?,
            default_font: None,
        })
    }

    pub fn with_default_font(mut self, path: impl Into<String>) -> Self {
        self.default_font = Some(path.into());
        self
    }

    pub fn spawn(&self, commands: &mut Commands<'_, '_>, asset_server: &AssetServer) -> Entity {
        let resources = SpawnResources {
            asset_server,
            stylesheet: &self.stylesheet,
            default_font: self.default_font.as_deref(),
        };
        let mut state = SpawnState::default();
        spawn_node(
            commands,
            &self.document.root,
            &resources,
            &mut state,
            None,
            None,
        )
    }
}

pub fn spawn_document(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    document: &UiDocument,
    stylesheet: &StyleSheet,
    default_font: &str,
) -> Entity {
    let resources = SpawnResources {
        asset_server,
        stylesheet,
        default_font: Some(default_font),
    };
    let mut state = SpawnState::default();
    spawn_node(commands, &document.root, &resources, &mut state, None, None)
}

pub fn spawn_document_with_embedded_font(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    document: &UiDocument,
    stylesheet: &StyleSheet,
) -> Entity {
    let resources = SpawnResources {
        asset_server,
        stylesheet,
        default_font: None,
    };
    let mut state = SpawnState::default();
    spawn_node(commands, &document.root, &resources, &mut state, None, None)
}

struct SpawnResources<'a> {
    asset_server: &'a AssetServer,
    stylesheet: &'a StyleSheet,
    default_font: Option<&'a str>,
}

#[derive(Default)]
struct SpawnState<'a> {
    ancestors: Vec<&'a ElementNode>,
    document_order: usize,
}

fn spawn_node<'a>(
    commands: &mut Commands<'_, '_>,
    node: &'a ElementNode,
    resources: &SpawnResources<'_>,
    state: &mut SpawnState<'a>,
    parent_entity: Option<Entity>,
    current_scope: Option<Entity>,
) -> Entity {
    let order = state.document_order;
    state.document_order += 1;
    let mut path = state.ancestors.clone();
    path.push(node);
    let style = resources.stylesheet.runtime_base_style_for_path(&path);
    let hover_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::Hover);
    let active_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::Active);
    let focus_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::Focus);
    let checked_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::Checked);
    let focus_within_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::FocusWithin);
    let focus_visible_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::FocusVisible);
    let ancestor_checked_style = resources
        .stylesheet
        .runtime_ancestor_state_style_for_path(&path, PseudoClass::Checked);
    let ancestor_focus_within_style = resources
        .stylesheet
        .runtime_ancestor_state_style_for_path(&path, PseudoClass::FocusWithin);
    let disabled_style = resources
        .stylesheet
        .runtime_state_style_for_path(&path, PseudoClass::Disabled);
    let disabled = UiXmlDisabled(node.attr("disabled").is_some());
    let runtime_state = UiXmlRuntimeState {
        disabled: disabled.0,
        ..Default::default()
    };
    let runtime_styles = RuntimeStyleInputs {
        base: &style,
        hover: &hover_style,
        active: &active_style,
        focus: &focus_style,
        checked: &checked_style,
        focus_within: &focus_within_style,
        focus_visible: &focus_visible_style,
        ancestor_checked: &ancestor_checked_style,
        ancestor_focus_within: &ancestor_focus_within_style,
        disabled: &disabled_style,
    };
    let style_source = UiXmlStyleSource::from_runtime_styles(runtime_styles);
    let selector_context = UiXmlSelectorContext::from_node(node, parent_entity, &state.ancestors);
    let bevy_style = to_bevy_style(&style);
    let background = style_color(style.background.as_deref(), Color::NONE, style.opacity);
    let border_color = style_color(style.border_color.as_deref(), Color::NONE, style.opacity);
    let text_color = style_color(style.color.as_deref(), Color::WHITE, style.opacity);
    let font_size = style.font_size.unwrap_or(16.0);
    let visibility = style
        .visibility
        .map(VisibilityValue::to_bevy)
        .unwrap_or_default();
    let z_index = style.z_index.map(ZIndex::Local).unwrap_or_default();

    let widget_type = node.widget_type();

    match widget_type {
        "button" | "checkbox" | "radio" => {
            let entity = commands
                .spawn(ButtonBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    border_color: border_color.into(),
                    visibility,
                    z_index,
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .insert(disabled)
                .insert(runtime_state)
                .insert(style_source)
                .insert(selector_context)
                .insert(UiXmlStateStyles::from_runtime_styles(runtime_styles))
                .id();
            attach_optional_render_components(commands, entity, &style);
            attach_widget_metadata(commands, entity, node, current_scope, order);

            let label = node.attr("label").unwrap_or(&node.text);
            if !label.trim().is_empty() {
                let font = load_font(resources.asset_server, resources.default_font);
                commands.entity(entity).with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        label.trim().to_string(),
                        TextStyle {
                            font,
                            font_size,
                            color: text_color,
                        },
                    ));
                });
            }

            add_children(
                commands,
                entity,
                node,
                resources,
                state,
                scope_for_children(node, entity, current_scope),
            );
            entity
        }
        "text-input" => {
            let entity = commands
                .spawn(ButtonBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    border_color: border_color.into(),
                    visibility,
                    z_index,
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .insert(disabled)
                .insert(runtime_state)
                .insert(style_source)
                .insert(selector_context)
                .insert(UiXmlStateStyles::from_runtime_styles(runtime_styles))
                .insert(UiXmlTextInput)
                .id();
            attach_optional_render_components(commands, entity, &style);
            attach_widget_metadata(commands, entity, node, current_scope, order);

            let font = load_font(resources.asset_server, resources.default_font);
            let text_value = UiXmlTextValue(node.attr("value").unwrap_or_default().to_string());
            let text_cursor = UiXmlTextCursor {
                position: text_value.0.chars().count(),
            };
            let placeholder = text_placeholder(node, &style, text_color, font_size);
            let mut display_entity = None;
            commands.entity(entity).with_children(|parent| {
                let mut section = TextSection {
                    value: String::new(),
                    style: TextStyle {
                        font,
                        font_size,
                        color: text_color,
                    },
                };
                apply_text_presentation(&mut section, &text_value, placeholder.as_ref());
                display_entity = Some(parent.spawn(TextBundle::from_sections([section])).id());
            });
            commands.entity(entity).insert(text_value);
            commands.entity(entity).insert(text_cursor);
            commands.entity(entity).insert(UiXmlTextSelection {
                anchor: text_cursor.position,
                focus: text_cursor.position,
            });
            commands.entity(entity).insert(UiXmlImePreedit::default());
            commands.entity(entity).insert(UiXmlInitialTextValue(
                node.attr("value").unwrap_or_default().to_string(),
            ));
            if let Some(placeholder) = placeholder {
                commands.entity(entity).insert(placeholder);
            }
            if let Some(display_entity) = display_entity {
                commands
                    .entity(entity)
                    .insert(UiXmlTextDisplay(display_entity));
            }

            entity
        }
        "text" => {
            let font = load_font(resources.asset_server, resources.default_font);
            let mut text = Text::from_section(
                node.attr("content")
                    .unwrap_or(&node.text)
                    .trim()
                    .to_string(),
                TextStyle {
                    font,
                    font_size,
                    color: text_color,
                },
            );
            if let Some(text_align) = style.text_align {
                text.justify = text_align.to_bevy();
            }
            if let Some(text_wrap) = style.text_wrap {
                text.linebreak_behavior = text_wrap.to_bevy();
            }
            let entity = commands
                .spawn(TextBundle {
                    text,
                    style: bevy_style,
                    background_color: background.into(),
                    visibility,
                    z_index,
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .insert(disabled)
                .insert(runtime_state)
                .insert(style_source)
                .insert(selector_context)
                .id();
            attach_optional_render_components(commands, entity, &style);
            attach_widget_metadata(commands, entity, node, current_scope, order);
            entity
        }
        "image" => {
            let image = node
                .attr("src")
                .map(|src| resources.asset_server.load(src.to_string()));
            let entity = commands
                .spawn(ImageBundle {
                    style: bevy_style,
                    image: image.map(UiImage::new).unwrap_or_default(),
                    background_color: background.into(),
                    visibility,
                    z_index,
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .insert(disabled)
                .insert(runtime_state)
                .insert(style_source)
                .insert(selector_context)
                .id();
            attach_optional_render_components(commands, entity, &style);
            attach_widget_metadata(commands, entity, node, current_scope, order);
            entity
        }
        _ => {
            let entity = commands
                .spawn(NodeBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    border_color: border_color.into(),
                    visibility,
                    z_index,
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .insert(disabled)
                .insert(runtime_state)
                .insert(style_source)
                .insert(selector_context)
                .id();
            attach_optional_render_components(commands, entity, &style);
            attach_widget_metadata(commands, entity, node, current_scope, order);
            add_children(
                commands,
                entity,
                node,
                resources,
                state,
                scope_for_children(node, entity, current_scope),
            );
            entity
        }
    }
}

fn text_placeholder(
    node: &ElementNode,
    style: &UiStyle,
    value_color: Color,
    value_font_size: f32,
) -> Option<UiXmlTextPlaceholder> {
    let placeholder_style = style.placeholder.as_deref();
    let text = node
        .attr("placeholder")
        .map(str::to_string)
        .unwrap_or_default();
    if text.is_empty() && placeholder_style.is_none() {
        return None;
    }

    Some(UiXmlTextPlaceholder {
        text,
        placeholder_color: placeholder_style
            .and_then(|style| style.color.as_deref())
            .map(|color| {
                style_color(
                    Some(color),
                    value_color,
                    placeholder_style.and_then(|s| s.opacity),
                )
            }),
        placeholder_font_size: placeholder_style.and_then(|style| style.font_size),
        value_color,
        value_font_size,
    })
}

fn add_children<'a>(
    commands: &mut Commands<'_, '_>,
    parent: Entity,
    node: &'a ElementNode,
    resources: &SpawnResources<'_>,
    state: &mut SpawnState<'a>,
    current_scope: Option<Entity>,
) {
    state.ancestors.push(node);
    for child in &node.children {
        let child_entity = spawn_node(
            commands,
            child,
            resources,
            state,
            Some(parent),
            current_scope,
        );
        commands.entity(parent).add_child(child_entity);
    }
    state.ancestors.pop();
}

fn attach_optional_render_components(
    commands: &mut Commands<'_, '_>,
    entity: Entity,
    style: &UiStyle,
) {
    if let Some(outline) = outline_from_style(style) {
        commands.entity(entity).insert(outline);
    }
    if let Some(effects) = unsupported_effects_from_style(style) {
        commands.entity(entity).insert(effects);
    }
    if let Some(colors) = border_colors_from_style(style) {
        commands.entity(entity).insert(colors);
    }
    if let Some(spec) = render_material_spec_from_style(style) {
        commands.entity(entity).insert(spec);
    }
}

fn attach_widget_metadata(
    commands: &mut Commands<'_, '_>,
    entity: Entity,
    node: &ElementNode,
    current_scope: Option<Entity>,
    order: usize,
) {
    commands.entity(entity).insert(UiXmlDocumentOrder(order));

    if node.widget_type() == "form" {
        commands.entity(entity).insert(UiXmlForm);
    }
    if node.attr("required").is_some() {
        commands.entity(entity).insert(UiXmlRequired(true));
        commands.entity(entity).insert(UiXmlValidationState {
            valid: true,
            reason: None,
        });
    }

    let Some(kind) = control_kind(node) else {
        attach_text_input_metadata(commands, entity, node, current_scope);
        return;
    };

    let scope = current_scope.unwrap_or(entity);
    let mut entity_commands = commands.entity(entity);
    entity_commands
        .insert(kind)
        .insert(UiXmlChecked(node.attr("checked").is_some()))
        .insert(UiXmlInitialChecked(node.attr("checked").is_some()))
        .insert(UiXmlControlValue(
            node.attr("value").unwrap_or("on").to_string(),
        ))
        .insert(UiXmlControlScope(scope));

    if let Some(name) = node
        .attr("name")
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        entity_commands.insert(UiXmlControlName(name.to_string()));
    }
}

fn attach_text_input_metadata(
    commands: &mut Commands<'_, '_>,
    entity: Entity,
    node: &ElementNode,
    current_scope: Option<Entity>,
) {
    if node.widget_type() != "text-input" {
        return;
    }

    let mut entity_commands = commands.entity(entity);
    entity_commands.insert(UiXmlControlScope(current_scope.unwrap_or(entity)));

    if let Some(name) = node
        .attr("name")
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
        entity_commands.insert(UiXmlControlName(name.to_string()));
    }
}

fn control_kind(node: &ElementNode) -> Option<UiXmlControlKind> {
    match node.widget_type() {
        "checkbox" => Some(UiXmlControlKind::Checkbox),
        "radio" => Some(UiXmlControlKind::Radio),
        _ => None,
    }
}

fn scope_for_children(
    node: &ElementNode,
    entity: Entity,
    current_scope: Option<Entity>,
) -> Option<Entity> {
    if node.widget_type() == "form" || current_scope.is_none() {
        Some(entity)
    } else {
        current_scope
    }
}

fn load_font(asset_server: &AssetServer, default_font: Option<&str>) -> Handle<Font> {
    default_font
        .map(|path| asset_server.load(path.to_string()))
        .unwrap_or_default()
}

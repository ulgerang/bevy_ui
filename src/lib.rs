//! Declarative Bevy UI from HTML-like XML and CSS-like JSON.
//!
//! The public surface intentionally mirrors the reference `ebitenui-xml`
//! project: load a layout string, load a style sheet string, then spawn a UI
//! tree into Bevy.

use bevy::prelude::*;
use roxmltree::Document;
use serde::Deserialize;
use std::collections::HashMap;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BevyUiXmlError {
    #[error("failed to parse XML layout: {0}")]
    Xml(#[from] roxmltree::Error),
    #[error("failed to parse JSON styles: {0}")]
    Json(#[from] serde_json::Error),
    #[error("layout is empty")]
    EmptyLayout,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiDocument {
    pub root: ElementNode,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElementNode {
    pub tag: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub text: String,
    pub attributes: HashMap<String, String>,
    pub children: Vec<ElementNode>,
}

impl ElementNode {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    pub fn widget_type(&self) -> &str {
        canonical_tag(&self.tag)
    }
}

fn canonical_tag(tag: &str) -> &str {
    match tag {
        "ui" | "panel" | "div" | "container" => "panel",
        "button" | "btn" => "button",
        "text" | "label" | "span" | "p" => "text",
        "image" | "img" => "image",
        _ => tag,
    }
}

pub fn parse_layout(input: &str) -> Result<UiDocument, BevyUiXmlError> {
    let xml = Document::parse(input)?;
    let root = xml
        .root()
        .children()
        .find(|node| node.is_element())
        .ok_or(BevyUiXmlError::EmptyLayout)?;

    Ok(UiDocument {
        root: parse_element(root),
    })
}

fn parse_element(node: roxmltree::Node<'_, '_>) -> ElementNode {
    let mut attributes = HashMap::new();
    let mut id = None;
    let mut classes = Vec::new();

    for attr in node.attributes() {
        match attr.name() {
            "id" => id = Some(attr.value().to_string()),
            "class" => {
                classes = attr
                    .value()
                    .split_whitespace()
                    .map(ToOwned::to_owned)
                    .collect();
            }
            name => {
                attributes.insert(name.to_string(), attr.value().to_string());
            }
        }
    }

    let children = node
        .children()
        .filter(|child| child.is_element())
        .map(parse_element)
        .collect();

    let text = node
        .children()
        .filter(|child| child.is_text())
        .filter_map(|child| child.text())
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" ");

    ElementNode {
        tag: node.tag_name().name().to_ascii_lowercase(),
        id,
        classes,
        text,
        attributes,
        children,
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StyleSheet {
    #[serde(default)]
    pub styles: HashMap<String, UiStyle>,
}

impl StyleSheet {
    pub fn parse(input: &str) -> Result<Self, BevyUiXmlError> {
        let value: serde_json::Value = serde_json::from_str(input)?;
        if value.get("styles").is_some() {
            Ok(serde_json::from_value(value)?)
        } else {
            Ok(Self {
                styles: serde_json::from_value(value)?,
            })
        }
    }

    pub fn computed_style(&self, node: &ElementNode) -> UiStyle {
        let mut style = UiStyle::default();
        let widget_type = node.widget_type();

        if let Some(tag_style) = self.styles.get(widget_type) {
            style.merge(tag_style);
        }

        if widget_type != node.tag {
            if let Some(tag_style) = self.styles.get(&node.tag) {
                style.merge(tag_style);
            }
        }

        for class_name in &node.classes {
            if let Some(class_style) = self.styles.get(&format!(".{class_name}")) {
                style.merge(class_style);
            }
        }

        if let Some(id) = &node.id {
            if let Some(id_style) = self.styles.get(&format!("#{id}")) {
                style.merge(id_style);
            }
        }

        style.apply_inline_attributes(node);
        style
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiStyle {
    pub width: Option<Length>,
    pub height: Option<Length>,
    pub min_width: Option<Length>,
    pub min_height: Option<Length>,
    pub max_width: Option<Length>,
    pub max_height: Option<Length>,
    pub padding: Option<EdgeSizes>,
    pub margin: Option<EdgeSizes>,
    pub direction: Option<FlexDirectionValue>,
    pub justify: Option<JustifyValue>,
    pub justify_content: Option<JustifyValue>,
    pub align: Option<AlignValue>,
    pub align_items: Option<AlignValue>,
    pub gap: Option<f32>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub background: Option<String>,
    pub color: Option<String>,
    pub font_size: Option<f32>,
    pub display: Option<DisplayValue>,
}

impl UiStyle {
    pub fn merge(&mut self, other: &UiStyle) {
        macro_rules! merge_field {
            ($field:ident) => {
                if other.$field.is_some() {
                    self.$field = other.$field.clone();
                }
            };
        }

        merge_field!(width);
        merge_field!(height);
        merge_field!(min_width);
        merge_field!(min_height);
        merge_field!(max_width);
        merge_field!(max_height);
        merge_field!(padding);
        merge_field!(margin);
        merge_field!(direction);
        merge_field!(justify);
        merge_field!(justify_content);
        merge_field!(align);
        merge_field!(align_items);
        merge_field!(gap);
        merge_field!(flex_grow);
        merge_field!(flex_shrink);
        merge_field!(background);
        merge_field!(color);
        merge_field!(font_size);
        merge_field!(display);
    }

    fn apply_inline_attributes(&mut self, node: &ElementNode) {
        if let Some(width) = node.attr("width").and_then(Length::parse) {
            self.width = Some(width);
        }
        if let Some(height) = node.attr("height").and_then(Length::parse) {
            self.height = Some(height);
        }
        if let Some(direction) = node.attr("direction").and_then(FlexDirectionValue::parse) {
            self.direction = Some(direction);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Length {
    Px(f32),
    Text(String),
}

impl Length {
    fn parse(value: &str) -> Option<Self> {
        if let Ok(px) = value.parse::<f32>() {
            return Some(Self::Px(px));
        }
        Some(Self::Text(value.to_string()))
    }

    fn into_val(self) -> Val {
        match self {
            Self::Px(value) => Val::Px(value),
            Self::Text(value) => {
                let trimmed = value.trim();
                if trimmed.eq_ignore_ascii_case("auto") {
                    Val::Auto
                } else if let Some(percent) = trimmed.strip_suffix('%') {
                    percent
                        .trim()
                        .parse::<f32>()
                        .map(Val::Percent)
                        .unwrap_or(Val::Auto)
                } else if let Some(px) = trimmed.strip_suffix("px") {
                    px.trim().parse::<f32>().map(Val::Px).unwrap_or(Val::Auto)
                } else {
                    trimmed.parse::<f32>().map(Val::Px).unwrap_or(Val::Auto)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum EdgeSizes {
    All(f32),
    Sides {
        all: Option<f32>,
        top: Option<f32>,
        right: Option<f32>,
        bottom: Option<f32>,
        left: Option<f32>,
    },
}

impl EdgeSizes {
    fn to_ui_rect(self) -> UiRect {
        match self {
            Self::All(value) => UiRect::all(Val::Px(value)),
            Self::Sides {
                all,
                top,
                right,
                bottom,
                left,
            } => {
                let fallback = all.unwrap_or(0.0);
                UiRect {
                    left: Val::Px(left.unwrap_or(fallback)),
                    right: Val::Px(right.unwrap_or(fallback)),
                    top: Val::Px(top.unwrap_or(fallback)),
                    bottom: Val::Px(bottom.unwrap_or(fallback)),
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FlexDirectionValue {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

impl FlexDirectionValue {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "row" => Some(Self::Row),
            "column" => Some(Self::Column),
            "row-reverse" => Some(Self::RowReverse),
            "column-reverse" => Some(Self::ColumnReverse),
            _ => None,
        }
    }

    fn to_bevy(self) -> FlexDirection {
        match self {
            Self::Row => FlexDirection::Row,
            Self::Column => FlexDirection::Column,
            Self::RowReverse => FlexDirection::RowReverse,
            Self::ColumnReverse => FlexDirection::ColumnReverse,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum JustifyValue {
    FlexStart,
    FlexEnd,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
}

impl JustifyValue {
    fn to_bevy(self) -> JustifyContent {
        match self {
            Self::FlexStart => JustifyContent::FlexStart,
            Self::FlexEnd => JustifyContent::FlexEnd,
            Self::Center => JustifyContent::Center,
            Self::SpaceBetween => JustifyContent::SpaceBetween,
            Self::SpaceAround => JustifyContent::SpaceAround,
            Self::SpaceEvenly => JustifyContent::SpaceEvenly,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AlignValue {
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
}

impl AlignValue {
    fn to_bevy(self) -> AlignItems {
        match self {
            Self::FlexStart => AlignItems::FlexStart,
            Self::FlexEnd => AlignItems::FlexEnd,
            Self::Center => AlignItems::Center,
            Self::Stretch => AlignItems::Stretch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DisplayValue {
    Flex,
    None,
}

impl DisplayValue {
    fn to_bevy(self) -> Display {
        match self {
            Self::Flex => Display::Flex,
            Self::None => Display::None,
        }
    }
}

pub struct UiXmlPlugin;

impl Plugin for UiXmlPlugin {
    fn build(&self, _app: &mut App) {}
}

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
        spawn_node(
            commands,
            asset_server,
            &self.document.root,
            &self.stylesheet,
            self.default_font.as_deref(),
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
    spawn_node(
        commands,
        asset_server,
        &document.root,
        stylesheet,
        Some(default_font),
    )
}

pub fn spawn_document_with_embedded_font(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    document: &UiDocument,
    stylesheet: &StyleSheet,
) -> Entity {
    spawn_node(commands, asset_server, &document.root, stylesheet, None)
}

fn spawn_node(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    node: &ElementNode,
    stylesheet: &StyleSheet,
    default_font: Option<&str>,
) -> Entity {
    let style = stylesheet.computed_style(node);
    let bevy_style = to_bevy_style(&style);
    let background = parse_color(style.background.as_deref()).unwrap_or(Color::NONE);
    let text_color = parse_color(style.color.as_deref()).unwrap_or(Color::WHITE);
    let font_size = style.font_size.unwrap_or(16.0);

    match node.widget_type() {
        "button" => {
            let entity = commands
                .spawn(ButtonBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    ..Default::default()
                })
                .id();

            let label = node.attr("label").unwrap_or(&node.text);
            if !label.trim().is_empty() {
                let font = load_font(asset_server, default_font);
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
                asset_server,
                entity,
                node,
                stylesheet,
                default_font,
            );
            entity
        }
        "text" => {
            let font = load_font(asset_server, default_font);
            commands
                .spawn(TextBundle {
                    text: Text::from_section(
                        node.attr("content")
                            .unwrap_or(&node.text)
                            .trim()
                            .to_string(),
                        TextStyle {
                            font,
                            font_size,
                            color: text_color,
                        },
                    ),
                    style: bevy_style,
                    ..Default::default()
                })
                .id()
        }
        "image" => {
            let image = node
                .attr("src")
                .map(|src| asset_server.load(src.to_string()));
            commands
                .spawn(ImageBundle {
                    style: bevy_style,
                    image: image.map(UiImage::new).unwrap_or_default(),
                    background_color: background.into(),
                    ..Default::default()
                })
                .id()
        }
        _ => {
            let entity = commands
                .spawn(NodeBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    ..Default::default()
                })
                .id();
            add_children(
                commands,
                asset_server,
                entity,
                node,
                stylesheet,
                default_font,
            );
            entity
        }
    }
}

fn add_children(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    parent: Entity,
    node: &ElementNode,
    stylesheet: &StyleSheet,
    default_font: Option<&str>,
) {
    for child in &node.children {
        let child_entity = spawn_node(commands, asset_server, child, stylesheet, default_font);
        commands.entity(parent).add_child(child_entity);
    }
}

fn load_font(asset_server: &AssetServer, default_font: Option<&str>) -> Handle<Font> {
    default_font
        .map(|path| asset_server.load(path.to_string()))
        .unwrap_or_default()
}

fn to_bevy_style(style: &UiStyle) -> Style {
    let mut bevy_style = Style::default();

    if let Some(width) = style.width.clone() {
        bevy_style.width = width.into_val();
    }
    if let Some(height) = style.height.clone() {
        bevy_style.height = height.into_val();
    }
    if let Some(min_width) = style.min_width.clone() {
        bevy_style.min_width = min_width.into_val();
    }
    if let Some(min_height) = style.min_height.clone() {
        bevy_style.min_height = min_height.into_val();
    }
    if let Some(max_width) = style.max_width.clone() {
        bevy_style.max_width = max_width.into_val();
    }
    if let Some(max_height) = style.max_height.clone() {
        bevy_style.max_height = max_height.into_val();
    }
    if let Some(padding) = style.padding {
        bevy_style.padding = padding.to_ui_rect();
    }
    if let Some(margin) = style.margin {
        bevy_style.margin = margin.to_ui_rect();
    }
    if let Some(direction) = style.direction {
        bevy_style.flex_direction = direction.to_bevy();
    }
    if let Some(justify) = style.justify_content.or(style.justify) {
        bevy_style.justify_content = justify.to_bevy();
    }
    if let Some(align) = style.align_items.or(style.align) {
        bevy_style.align_items = align.to_bevy();
    }
    if let Some(gap) = style.gap {
        bevy_style.row_gap = Val::Px(gap);
        bevy_style.column_gap = Val::Px(gap);
    }
    if let Some(flex_grow) = style.flex_grow {
        bevy_style.flex_grow = flex_grow;
    }
    if let Some(flex_shrink) = style.flex_shrink {
        bevy_style.flex_shrink = flex_shrink;
    }
    if let Some(display) = style.display {
        bevy_style.display = display.to_bevy();
    }

    bevy_style
}

fn parse_color(value: Option<&str>) -> Option<Color> {
    let value = value?.trim();
    if value.eq_ignore_ascii_case("transparent") {
        return Some(Color::NONE);
    }
    if let Some(hex) = value.strip_prefix('#') {
        return parse_hex_color(hex);
    }
    if let Some(color) = parse_rgb_function(value) {
        return Some(color);
    }
    if let Some(color) = parse_gradient_fallback(value) {
        return Some(color);
    }
    parse_named_color(value)
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            Some(Color::rgb_u8(r, g, b))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::rgb_u8(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some(Color::rgba_u8(r, g, b, a))
        }
        _ => None,
    }
}

fn parse_rgb_function(value: &str) -> Option<Color> {
    let value = value.trim();
    let (function, args) = value.split_once('(')?;
    let function = function.trim().to_ascii_lowercase();
    if function != "rgb" && function != "rgba" {
        return None;
    }

    let args = args.strip_suffix(')')?;
    let parts = split_css_args(args);
    if parts.len() < 3 || parts.len() > 4 {
        return None;
    }

    let r = parse_rgb_component(&parts[0])?;
    let g = parse_rgb_component(&parts[1])?;
    let b = parse_rgb_component(&parts[2])?;
    let a = parts
        .get(3)
        .and_then(|value| parse_alpha_component(value))
        .unwrap_or(1.0);

    Some(Color::rgba(
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        a,
    ))
}

fn parse_rgb_component(value: &str) -> Option<u8> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        let percent = percent.trim().parse::<f32>().ok()?.clamp(0.0, 100.0);
        return Some((percent * 2.55).round() as u8);
    }

    Some(value.parse::<f32>().ok()?.round().clamp(0.0, 255.0) as u8)
}

fn parse_alpha_component(value: &str) -> Option<f32> {
    let value = value.trim();
    if let Some(percent) = value.strip_suffix('%') {
        return Some((percent.trim().parse::<f32>().ok()? / 100.0).clamp(0.0, 1.0));
    }

    let alpha = value.parse::<f32>().ok()?;
    if alpha > 1.0 {
        Some((alpha / 255.0).clamp(0.0, 1.0))
    } else {
        Some(alpha.clamp(0.0, 1.0))
    }
}

fn parse_gradient_fallback(value: &str) -> Option<Color> {
    let value = value.trim();
    let (function, args) = value.split_once('(')?;
    let function = function.trim().to_ascii_lowercase();
    if function != "linear-gradient" && function != "radial-gradient" {
        return None;
    }

    let args = args.strip_suffix(')')?;
    split_css_args(args)
        .into_iter()
        .filter_map(|part| parse_color_stop(&part))
        .next()
}

fn parse_color_stop(value: &str) -> Option<Color> {
    let value = value.trim();
    if let Some(color) = parse_color(Some(value)) {
        return Some(color);
    }

    if let Some(end) = value.find(')') {
        if let Some(color) = parse_color(Some(&value[..=end])) {
            return Some(color);
        }
    }

    value
        .split_whitespace()
        .next()
        .and_then(|candidate| parse_color(Some(candidate)))
}

fn split_css_args(value: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;

    for (index, ch) in value.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                args.push(value[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    args.push(value[start..].trim().to_string());
    args
}

fn parse_named_color(value: &str) -> Option<Color> {
    match value.to_ascii_lowercase().as_str() {
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        "red" => Some(Color::rgb_u8(255, 0, 0)),
        "crimson" => Some(Color::rgb_u8(220, 20, 60)),
        "darkred" => Some(Color::rgb_u8(139, 0, 0)),
        "tomato" => Some(Color::rgb_u8(255, 99, 71)),
        "green" => Some(Color::rgb_u8(0, 128, 0)),
        "forestgreen" => Some(Color::rgb_u8(34, 139, 34)),
        "limegreen" => Some(Color::rgb_u8(50, 205, 50)),
        "blue" => Some(Color::rgb_u8(0, 0, 255)),
        "royalblue" => Some(Color::rgb_u8(65, 105, 225)),
        "yellow" => Some(Color::rgb_u8(255, 255, 0)),
        "gold" => Some(Color::rgb_u8(255, 215, 0)),
        "deepskyblue" => Some(Color::rgb_u8(0, 191, 255)),
        "lightgray" | "lightgrey" => Some(Color::rgb_u8(211, 211, 211)),
        "gray" | "grey" => Some(Color::rgb_u8(128, 128, 128)),
        "slategray" | "slategrey" => Some(Color::rgb_u8(112, 128, 144)),
        "cornflowerblue" => Some(Color::rgb_u8(100, 149, 237)),
        "dodgerblue" => Some(Color::rgb_u8(30, 144, 255)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}

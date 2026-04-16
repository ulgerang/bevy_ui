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

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlElement {
    pub tag: String,
    pub widget_type: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: HashMap<String, String>,
}

impl From<&ElementNode> for UiXmlElement {
    fn from(node: &ElementNode) -> Self {
        Self {
            tag: node.tag.clone(),
            widget_type: node.widget_type().to_string(),
            id: node.id.clone(),
            classes: node.classes.clone(),
            attributes: node.attributes.clone(),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct UiXmlStateStyles {
    base: UiStyle,
    hover: Option<UiStyle>,
    active: Option<UiStyle>,
    disabled: Option<UiStyle>,
}

impl UiXmlStateStyles {
    fn from_style(style: &UiStyle) -> Self {
        Self {
            base: style.without_state_styles(),
            hover: style.hover.as_deref().map(UiStyle::without_state_styles),
            active: style.active.as_deref().map(UiStyle::without_state_styles),
            disabled: style.disabled.as_deref().map(UiStyle::without_state_styles),
        }
    }

    fn resolve(&self, interaction: Interaction, disabled: bool) -> UiStyle {
        let mut style = self.base.clone();
        if disabled {
            if let Some(disabled) = &self.disabled {
                style.merge(disabled);
            }
            return style;
        }

        match interaction {
            Interaction::Pressed => {
                if let Some(active) = &self.active {
                    style.merge(active);
                }
            }
            Interaction::Hovered => {
                if let Some(hover) = &self.hover {
                    style.merge(hover);
                }
            }
            Interaction::None => {}
        }

        style
    }
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

#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub styles: HashMap<String, UiStyle>,
    pub diagnostics: Vec<StyleDiagnostic>,
    rules: Vec<StyleRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleDiagnostic {
    UnsupportedProperty { selector: String, property: String },
    InvalidSelector { selector: String, reason: String },
}

#[derive(Debug, Clone)]
struct StyleRule {
    selector: Selector,
    style: UiStyle,
    specificity: u32,
    order: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Selector {
    parts: Vec<SelectorPart>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Combinator {
    Descendant,
    Child,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SelectorPart {
    combinator: Option<Combinator>,
    simple: SimpleSelector,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct SimpleSelector {
    tag: Option<String>,
    id: Option<String>,
    classes: Vec<String>,
    attributes: Vec<AttributeSelector>,
    pseudo: Option<PseudoClass>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AttributeSelector {
    name: String,
    value: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PseudoClass {
    Hover,
    Active,
    Focus,
    Disabled,
}

impl Selector {
    fn parse(input: &str) -> Option<Self> {
        let parts = split_selector(input)
            .into_iter()
            .map(|(combinator, token)| {
                SimpleSelector::parse(&token).map(|simple| SelectorPart { combinator, simple })
            })
            .collect::<Option<Vec<_>>>()?;

        if parts.is_empty() {
            None
        } else {
            Some(Self { parts })
        }
    }

    fn specificity(&self) -> u32 {
        self.parts
            .iter()
            .map(|part| part.simple.specificity())
            .sum()
    }

    fn matches(&self, path: &[&ElementNode]) -> Option<u32> {
        if path.is_empty() {
            return None;
        }

        let mut path_index = path.len() - 1;
        let mut bonus = self.parts.last()?.simple.matches(path[path_index])?;

        for part_index in (0..self.parts.len().saturating_sub(1)).rev() {
            match self.parts[part_index + 1]
                .combinator
                .unwrap_or(Combinator::Descendant)
            {
                Combinator::Child => {
                    if path_index == 0 {
                        return None;
                    }
                    path_index -= 1;
                    bonus += self.parts[part_index].simple.matches(path[path_index])?;
                }
                Combinator::Descendant => {
                    let mut matched = None;
                    for ancestor_index in (0..path_index).rev() {
                        if let Some(part_bonus) =
                            self.parts[part_index].simple.matches(path[ancestor_index])
                        {
                            matched = Some((ancestor_index, part_bonus));
                            break;
                        }
                    }
                    let (ancestor_index, part_bonus) = matched?;
                    path_index = ancestor_index;
                    bonus += part_bonus;
                }
            }
        }

        Some(bonus)
    }
}

impl SimpleSelector {
    fn parse(input: &str) -> Option<Self> {
        let mut selector = Self::default();
        let chars = input.trim().chars().collect::<Vec<_>>();
        if chars.is_empty() {
            return None;
        }

        let mut index = 0;
        if is_ident_start(chars[index]) || chars[index] == '*' {
            let start = index;
            index += 1;
            while index < chars.len() && is_ident_continue(chars[index]) {
                index += 1;
            }
            if chars[start] != '*' {
                selector.tag = Some(
                    chars[start..index]
                        .iter()
                        .collect::<String>()
                        .to_ascii_lowercase(),
                );
            }
        }

        while index < chars.len() {
            match chars[index] {
                '#' => {
                    index += 1;
                    let (value, next) = parse_ident(&chars, index)?;
                    selector.id = Some(value);
                    index = next;
                }
                '.' => {
                    index += 1;
                    let (value, next) = parse_ident(&chars, index)?;
                    selector.classes.push(value);
                    index = next;
                }
                '[' => {
                    let (attribute, next) = parse_attribute_selector(&chars, index)?;
                    selector.attributes.push(attribute);
                    index = next;
                }
                ':' => {
                    index += 1;
                    let (value, next) = parse_ident(&chars, index)?;
                    selector.pseudo = PseudoClass::parse(&value);
                    selector.pseudo?;
                    index = next;
                }
                _ => return None,
            }
        }

        Some(selector)
    }

    fn specificity(&self) -> u32 {
        let mut score = 0;
        if self.id.is_some() {
            score += 100;
        }
        score += (self.classes.len() + self.attributes.len()) as u32 * 10;
        if self.pseudo.is_some() {
            score += 10;
        }
        if self.tag.is_some() {
            score += 1;
        }
        score
    }

    fn matches(&self, node: &ElementNode) -> Option<u32> {
        let mut bonus = 0;

        if let Some(tag) = &self.tag {
            if tag == &node.tag {
                if node.widget_type() != node.tag {
                    bonus += 1;
                }
            } else if tag != node.widget_type() {
                return None;
            }
        }

        if let Some(id) = &self.id {
            if node.id.as_deref() != Some(id.as_str()) {
                return None;
            }
        }

        for class_name in &self.classes {
            if !node.classes.iter().any(|candidate| candidate == class_name) {
                return None;
            }
        }

        for attr in &self.attributes {
            let value = node.attr(&attr.name)?;
            if let Some(expected) = &attr.value {
                if value != expected {
                    return None;
                }
            }
        }

        if let Some(pseudo) = self.pseudo {
            if !pseudo.matches_static(node) {
                return None;
            }
        }

        Some(bonus)
    }
}

impl PseudoClass {
    fn parse(input: &str) -> Option<Self> {
        match input {
            "hover" => Some(Self::Hover),
            "active" => Some(Self::Active),
            "focus" => Some(Self::Focus),
            "disabled" => Some(Self::Disabled),
            _ => None,
        }
    }

    fn matches_static(self, node: &ElementNode) -> bool {
        match self {
            Self::Disabled => node.attr("disabled").is_some(),
            Self::Hover | Self::Active | Self::Focus => false,
        }
    }
}

fn split_selector(input: &str) -> Vec<(Option<Combinator>, String)> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let mut pending = None;

    for ch in input.trim().chars() {
        match ch {
            '[' => {
                depth += 1;
                current.push(ch);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                current.push(ch);
            }
            '>' if depth == 0 => {
                push_selector_part(&mut parts, &mut current, pending.take());
                pending = Some(Combinator::Child);
            }
            ch if ch.is_whitespace() && depth == 0 => {
                if current.trim().is_empty() {
                    if !parts.is_empty() && pending.is_none() {
                        pending = Some(Combinator::Descendant);
                    }
                } else {
                    push_selector_part(&mut parts, &mut current, pending.take());
                    if !parts.is_empty() && pending.is_none() {
                        pending = Some(Combinator::Descendant);
                    }
                }
            }
            _ => current.push(ch),
        }
    }

    push_selector_part(&mut parts, &mut current, pending);
    if let Some(first) = parts.first_mut() {
        first.0 = None;
    }
    parts
}

fn push_selector_part(
    parts: &mut Vec<(Option<Combinator>, String)>,
    current: &mut String,
    combinator: Option<Combinator>,
) {
    let token = current.trim();
    if !token.is_empty() {
        parts.push((combinator, token.to_string()));
    }
    current.clear();
}

fn parse_ident(chars: &[char], start: usize) -> Option<(String, usize)> {
    if start >= chars.len() || !is_ident_start(chars[start]) {
        return None;
    }

    let mut index = start + 1;
    while index < chars.len() && is_ident_continue(chars[index]) {
        index += 1;
    }

    Some((chars[start..index].iter().collect(), index))
}

fn parse_attribute_selector(chars: &[char], start: usize) -> Option<(AttributeSelector, usize)> {
    let mut index = start + 1;
    let mut raw = String::new();
    while index < chars.len() && chars[index] != ']' {
        raw.push(chars[index]);
        index += 1;
    }
    if index >= chars.len() {
        return None;
    }

    let (name, value) = if let Some((name, value)) = raw.split_once('=') {
        (
            name.trim().to_string(),
            Some(
                value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string(),
            ),
        )
    } else {
        (raw.trim().to_string(), None)
    };

    if name.is_empty() {
        return None;
    }

    Some((AttributeSelector { name, value }, index + 1))
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '-'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '-'
}

fn normalize_style_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .iter()
                .map(|(key, value)| {
                    (
                        canonical_property_name(key).unwrap_or(key).to_string(),
                        normalize_style_value(value),
                    )
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(normalize_style_value).collect())
        }
        value => value.clone(),
    }
}

fn collect_style_diagnostics(
    selector: &str,
    value: &serde_json::Value,
    diagnostics: &mut Vec<StyleDiagnostic>,
) {
    let Some(object) = value.as_object() else {
        return;
    };

    for (property, nested) in object {
        if let Some(state) = matches_state_property(property) {
            collect_style_diagnostics(selector, nested, diagnostics);
            if canonical_property_name(state).is_none() {
                diagnostics.push(StyleDiagnostic::UnsupportedProperty {
                    selector: selector.to_string(),
                    property: property.clone(),
                });
            }
            continue;
        }

        if canonical_property_name(property).is_none() {
            diagnostics.push(StyleDiagnostic::UnsupportedProperty {
                selector: selector.to_string(),
                property: property.clone(),
            });
        }
    }
}

fn matches_state_property(property: &str) -> Option<&str> {
    match property {
        "hover" | "active" | "focus" | "disabled" => Some(property),
        _ => None,
    }
}

fn canonical_property_name(property: &str) -> Option<&'static str> {
    match property {
        "width" => Some("width"),
        "height" => Some("height"),
        "minWidth" | "min-width" => Some("minWidth"),
        "minHeight" | "min-height" => Some("minHeight"),
        "maxWidth" | "max-width" => Some("maxWidth"),
        "maxHeight" | "max-height" => Some("maxHeight"),
        "padding" => Some("padding"),
        "margin" => Some("margin"),
        "border" => Some("border"),
        "borderWidth" | "border-width" => Some("borderWidth"),
        "borderColor" | "border-color" => Some("borderColor"),
        "position" => Some("position"),
        "left" => Some("left"),
        "right" => Some("right"),
        "top" => Some("top"),
        "bottom" => Some("bottom"),
        "overflow" => Some("overflow"),
        "aspectRatio" | "aspect-ratio" => Some("aspectRatio"),
        "direction" => Some("direction"),
        "flexDirection" | "flex-direction" => Some("flexDirection"),
        "flexWrap" | "flex-wrap" => Some("flexWrap"),
        "justify" => Some("justify"),
        "justifyContent" | "justify-content" => Some("justifyContent"),
        "align" => Some("align"),
        "alignItems" | "align-items" => Some("alignItems"),
        "alignSelf" | "align-self" => Some("alignSelf"),
        "gap" => Some("gap"),
        "rowGap" | "row-gap" => Some("rowGap"),
        "columnGap" | "column-gap" => Some("columnGap"),
        "flexGrow" | "flex-grow" => Some("flexGrow"),
        "flexShrink" | "flex-shrink" => Some("flexShrink"),
        "flexBasis" | "flex-basis" => Some("flexBasis"),
        "background" | "backgroundColor" | "background-color" => Some("background"),
        "color" => Some("color"),
        "fontSize" | "font-size" => Some("fontSize"),
        "opacity" => Some("opacity"),
        "display" => Some("display"),
        "hover" => Some("hover"),
        "active" => Some("active"),
        "focus" => Some("focus"),
        "disabled" => Some("disabled"),
        _ => None,
    }
}

impl StyleSheet {
    pub fn parse(input: &str) -> Result<Self, BevyUiXmlError> {
        let value: serde_json::Value = serde_json::from_str(input)?;

        let styles_value = value.get("styles").unwrap_or(&value);
        let Some(styles_object) = styles_value.as_object() else {
            return Ok(Self::default());
        };

        let mut styles = HashMap::new();
        let mut diagnostics = Vec::new();
        let mut rules = Vec::new();

        for (order, (selector, style_value)) in styles_object.iter().enumerate() {
            collect_style_diagnostics(selector, style_value, &mut diagnostics);
            let style: UiStyle = serde_json::from_value(normalize_style_value(style_value))?;
            styles.insert(selector.clone(), style.clone());

            match Selector::parse(selector) {
                Some(selector) => rules.push(StyleRule {
                    specificity: selector.specificity(),
                    order,
                    selector,
                    style,
                }),
                None => diagnostics.push(StyleDiagnostic::InvalidSelector {
                    selector: selector.clone(),
                    reason: "empty selector".to_string(),
                }),
            }
        }

        Ok(Self {
            styles,
            diagnostics,
            rules,
        })
    }

    pub fn computed_style(&self, node: &ElementNode) -> UiStyle {
        self.computed_style_for_path(&[node])
    }

    pub fn computed_style_for_path(&self, path: &[&ElementNode]) -> UiStyle {
        if self.rules.is_empty() {
            self.legacy_computed_style(path.last().copied())
        } else {
            self.rule_computed_style(path)
        }
    }

    fn legacy_computed_style(&self, node: Option<&ElementNode>) -> UiStyle {
        let Some(node) = node else {
            return UiStyle::default();
        };
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

    fn rule_computed_style(&self, path: &[&ElementNode]) -> UiStyle {
        let Some(node) = path.last().copied() else {
            return UiStyle::default();
        };

        let mut matched = self
            .rules
            .iter()
            .filter_map(|rule| {
                rule.selector
                    .matches(path)
                    .map(|bonus| (rule.specificity + bonus, rule.order, &rule.style))
            })
            .collect::<Vec<_>>();

        matched.sort_by_key(|(specificity, order, _)| (*specificity, *order));

        let mut style = UiStyle::default();
        for (_, _, rule_style) in matched {
            style.merge(rule_style);
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
    pub border: Option<EdgeSizes>,
    pub border_width: Option<EdgeSizes>,
    pub border_color: Option<String>,
    pub position: Option<PositionValue>,
    pub left: Option<Length>,
    pub right: Option<Length>,
    pub top: Option<Length>,
    pub bottom: Option<Length>,
    pub overflow: Option<OverflowValue>,
    pub aspect_ratio: Option<f32>,
    pub direction: Option<FlexDirectionValue>,
    pub flex_direction: Option<FlexDirectionValue>,
    pub flex_wrap: Option<FlexWrapValue>,
    pub justify: Option<JustifyValue>,
    pub justify_content: Option<JustifyValue>,
    pub align: Option<AlignValue>,
    pub align_items: Option<AlignValue>,
    pub align_self: Option<AlignSelfValue>,
    pub gap: Option<f32>,
    pub row_gap: Option<Length>,
    pub column_gap: Option<Length>,
    pub flex_grow: Option<f32>,
    pub flex_shrink: Option<f32>,
    pub flex_basis: Option<Length>,
    pub background: Option<String>,
    pub color: Option<String>,
    pub font_size: Option<f32>,
    pub opacity: Option<f32>,
    pub display: Option<DisplayValue>,
    pub hover: Option<Box<UiStyle>>,
    pub active: Option<Box<UiStyle>>,
    pub focus: Option<Box<UiStyle>>,
    pub disabled: Option<Box<UiStyle>>,
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
        merge_field!(border);
        merge_field!(border_width);
        merge_field!(border_color);
        merge_field!(position);
        merge_field!(left);
        merge_field!(right);
        merge_field!(top);
        merge_field!(bottom);
        merge_field!(overflow);
        merge_field!(aspect_ratio);
        merge_field!(direction);
        merge_field!(flex_direction);
        merge_field!(flex_wrap);
        merge_field!(justify);
        merge_field!(justify_content);
        merge_field!(align);
        merge_field!(align_items);
        merge_field!(align_self);
        merge_field!(gap);
        merge_field!(row_gap);
        merge_field!(column_gap);
        merge_field!(flex_grow);
        merge_field!(flex_shrink);
        merge_field!(flex_basis);
        merge_field!(background);
        merge_field!(color);
        merge_field!(font_size);
        merge_field!(opacity);
        merge_field!(display);
        merge_field!(hover);
        merge_field!(active);
        merge_field!(focus);
        merge_field!(disabled);
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

    fn without_state_styles(&self) -> Self {
        let mut style = self.clone();
        style.hover = None;
        style.active = None;
        style.focus = None;
        style.disabled = None;
        style
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum EdgeSizes {
    All(Length),
    Array(Vec<Length>),
    Sides {
        all: Option<Length>,
        x: Option<Length>,
        y: Option<Length>,
        top: Option<Length>,
        right: Option<Length>,
        bottom: Option<Length>,
        left: Option<Length>,
    },
}

impl EdgeSizes {
    fn to_ui_rect(&self) -> UiRect {
        match self {
            Self::All(value) => UiRect::all(value.clone().into_val()),
            Self::Array(values) => match values.as_slice() {
                [all] => UiRect::all(all.clone().into_val()),
                [vertical, horizontal] => {
                    UiRect::axes(horizontal.clone().into_val(), vertical.clone().into_val())
                }
                [top, horizontal, bottom] => UiRect {
                    left: horizontal.clone().into_val(),
                    right: horizontal.clone().into_val(),
                    top: top.clone().into_val(),
                    bottom: bottom.clone().into_val(),
                },
                [top, right, bottom, left, ..] => UiRect {
                    left: left.clone().into_val(),
                    right: right.clone().into_val(),
                    top: top.clone().into_val(),
                    bottom: bottom.clone().into_val(),
                },
                [] => UiRect::default(),
            },
            Self::Sides {
                all,
                x,
                y,
                top,
                right,
                bottom,
                left,
            } => {
                let fallback = all.clone().unwrap_or(Length::Px(0.0));
                let horizontal = x.clone().unwrap_or_else(|| fallback.clone());
                let vertical = y.clone().unwrap_or_else(|| fallback.clone());
                UiRect {
                    left: left
                        .clone()
                        .unwrap_or_else(|| horizontal.clone())
                        .into_val(),
                    right: right
                        .clone()
                        .unwrap_or_else(|| horizontal.clone())
                        .into_val(),
                    top: top.clone().unwrap_or_else(|| vertical.clone()).into_val(),
                    bottom: bottom
                        .clone()
                        .unwrap_or_else(|| vertical.clone())
                        .into_val(),
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
pub enum FlexWrapValue {
    NoWrap,
    Wrap,
    WrapReverse,
}

impl FlexWrapValue {
    fn to_bevy(self) -> FlexWrap {
        match self {
            Self::NoWrap => FlexWrap::NoWrap,
            Self::Wrap => FlexWrap::Wrap,
            Self::WrapReverse => FlexWrap::WrapReverse,
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
pub enum AlignSelfValue {
    Auto,
    FlexStart,
    FlexEnd,
    Center,
    Stretch,
}

impl AlignSelfValue {
    fn to_bevy(self) -> AlignSelf {
        match self {
            Self::Auto => AlignSelf::Auto,
            Self::FlexStart => AlignSelf::FlexStart,
            Self::FlexEnd => AlignSelf::FlexEnd,
            Self::Center => AlignSelf::Center,
            Self::Stretch => AlignSelf::Stretch,
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

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PositionValue {
    Relative,
    Absolute,
}

impl PositionValue {
    fn to_bevy(self) -> PositionType {
        match self {
            Self::Relative => PositionType::Relative,
            Self::Absolute => PositionType::Absolute,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OverflowValue {
    Visible,
    Hidden,
    Clip,
}

impl OverflowValue {
    fn to_bevy(self) -> Overflow {
        match self {
            Self::Visible => Overflow::visible(),
            Self::Hidden | Self::Clip => Overflow::clip(),
        }
    }
}

pub struct UiXmlPlugin;

impl Plugin for UiXmlPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, apply_interaction_styles);
    }
}

type InteractionStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static Interaction,
        &'static UiXmlStateStyles,
        &'static UiXmlElement,
        &'static mut Style,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
    ),
    Changed<Interaction>,
>;

fn apply_interaction_styles(mut query: InteractionStyleQuery<'_, '_>) {
    for (interaction, state_styles, element, mut bevy_style, mut background, mut border) in
        &mut query
    {
        let resolved =
            state_styles.resolve(*interaction, element.attributes.contains_key("disabled"));
        *bevy_style = to_bevy_style(&resolved);
        background.0 = style_color(
            resolved.background.as_deref(),
            Color::NONE,
            resolved.opacity,
        );
        border.0 = style_color(
            resolved.border_color.as_deref(),
            Color::NONE,
            resolved.opacity,
        );
    }
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
        let mut ancestors = Vec::new();
        spawn_node(
            commands,
            asset_server,
            &self.document.root,
            &self.stylesheet,
            self.default_font.as_deref(),
            &mut ancestors,
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
    let mut ancestors = Vec::new();
    spawn_node(
        commands,
        asset_server,
        &document.root,
        stylesheet,
        Some(default_font),
        &mut ancestors,
    )
}

pub fn spawn_document_with_embedded_font(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    document: &UiDocument,
    stylesheet: &StyleSheet,
) -> Entity {
    let mut ancestors = Vec::new();
    spawn_node(
        commands,
        asset_server,
        &document.root,
        stylesheet,
        None,
        &mut ancestors,
    )
}

fn spawn_node<'a>(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    node: &'a ElementNode,
    stylesheet: &StyleSheet,
    default_font: Option<&str>,
    ancestors: &mut Vec<&'a ElementNode>,
) -> Entity {
    let mut path = ancestors.clone();
    path.push(node);
    let style = stylesheet.computed_style_for_path(&path);
    let bevy_style = to_bevy_style(&style);
    let background = style_color(style.background.as_deref(), Color::NONE, style.opacity);
    let border_color = style_color(style.border_color.as_deref(), Color::NONE, style.opacity);
    let text_color = style_color(style.color.as_deref(), Color::WHITE, style.opacity);
    let font_size = style.font_size.unwrap_or(16.0);

    match node.widget_type() {
        "button" => {
            let entity = commands
                .spawn(ButtonBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    border_color: border_color.into(),
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .insert(UiXmlStateStyles::from_style(&style))
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
                ancestors,
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
                    background_color: background.into(),
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
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
                .insert(UiXmlElement::from(node))
                .id()
        }
        _ => {
            let entity = commands
                .spawn(NodeBundle {
                    style: bevy_style,
                    background_color: background.into(),
                    border_color: border_color.into(),
                    ..Default::default()
                })
                .insert(UiXmlElement::from(node))
                .id();
            add_children(
                commands,
                asset_server,
                entity,
                node,
                stylesheet,
                default_font,
                ancestors,
            );
            entity
        }
    }
}

fn add_children<'a>(
    commands: &mut Commands<'_, '_>,
    asset_server: &AssetServer,
    parent: Entity,
    node: &'a ElementNode,
    stylesheet: &StyleSheet,
    default_font: Option<&str>,
    ancestors: &mut Vec<&'a ElementNode>,
) {
    ancestors.push(node);
    for child in &node.children {
        let child_entity = spawn_node(
            commands,
            asset_server,
            child,
            stylesheet,
            default_font,
            ancestors,
        );
        commands.entity(parent).add_child(child_entity);
    }
    ancestors.pop();
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
    if let Some(position) = style.position {
        bevy_style.position_type = position.to_bevy();
    }
    if let Some(left) = style.left.clone() {
        bevy_style.left = left.into_val();
    }
    if let Some(right) = style.right.clone() {
        bevy_style.right = right.into_val();
    }
    if let Some(top) = style.top.clone() {
        bevy_style.top = top.into_val();
    }
    if let Some(bottom) = style.bottom.clone() {
        bevy_style.bottom = bottom.into_val();
    }
    if let Some(overflow) = style.overflow {
        bevy_style.overflow = overflow.to_bevy();
    }
    if let Some(aspect_ratio) = style.aspect_ratio {
        bevy_style.aspect_ratio = Some(aspect_ratio);
    }
    if let Some(padding) = &style.padding {
        bevy_style.padding = padding.to_ui_rect();
    }
    if let Some(margin) = &style.margin {
        bevy_style.margin = margin.to_ui_rect();
    }
    if let Some(border) = &style.border_width {
        bevy_style.border = border.to_ui_rect();
    } else if let Some(border) = &style.border {
        bevy_style.border = border.to_ui_rect();
    }
    if let Some(direction) = style.flex_direction.or(style.direction) {
        bevy_style.flex_direction = direction.to_bevy();
    }
    if let Some(flex_wrap) = style.flex_wrap {
        bevy_style.flex_wrap = flex_wrap.to_bevy();
    }
    if let Some(justify) = style.justify_content.or(style.justify) {
        bevy_style.justify_content = justify.to_bevy();
    }
    if let Some(align) = style.align_items.or(style.align) {
        bevy_style.align_items = align.to_bevy();
    }
    if let Some(align_self) = style.align_self {
        bevy_style.align_self = align_self.to_bevy();
    }
    if let Some(gap) = style.gap {
        bevy_style.row_gap = Val::Px(gap);
        bevy_style.column_gap = Val::Px(gap);
    }
    if let Some(row_gap) = style.row_gap.clone() {
        bevy_style.row_gap = row_gap.into_val();
    }
    if let Some(column_gap) = style.column_gap.clone() {
        bevy_style.column_gap = column_gap.into_val();
    }
    if let Some(flex_grow) = style.flex_grow {
        bevy_style.flex_grow = flex_grow;
    }
    if let Some(flex_shrink) = style.flex_shrink {
        bevy_style.flex_shrink = flex_shrink;
    }
    if let Some(flex_basis) = style.flex_basis.clone() {
        bevy_style.flex_basis = flex_basis.into_val();
    }
    if let Some(display) = style.display {
        bevy_style.display = display.to_bevy();
    }

    bevy_style
}

fn style_color(value: Option<&str>, fallback: Color, opacity: Option<f32>) -> Color {
    let mut color = parse_color(value).unwrap_or(fallback);
    if let Some(opacity) = opacity {
        let [r, g, b, a] = color.as_rgba_f32();
        color = Color::rgba(r, g, b, a * opacity.clamp(0.0, 1.0));
    }
    color
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
                        "flex-basis": "25%"
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
            StyleDiagnostic::UnsupportedProperty { property, .. } if property == "boxShadow"
        )));
        assert!(sheet.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic,
            StyleDiagnostic::UnsupportedProperty { property, .. } if property == "filter"
        )));
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
}

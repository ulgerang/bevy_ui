use crate::selector::{PseudoClass, Selector};
use crate::{BevyUiXmlError, ElementNode};
use bevy::prelude::*;
use bevy::text::BreakLineOn;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    pub styles: HashMap<String, UiStyle>,
    pub diagnostics: Vec<StyleDiagnostic>,
    rules: Vec<StyleRule>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StyleDiagnostic {
    UnsupportedProperty {
        selector: String,
        property: String,
    },
    UnsupportedEffect {
        selector: String,
        property: String,
        reason: String,
    },
    InvalidSelector {
        selector: String,
        reason: String,
    },
    UnresolvedVariable {
        selector: String,
        property: String,
        variable: String,
    },
}

#[derive(Debug, Clone)]
struct StyleRule {
    selector: Selector,
    style: UiStyle,
    specificity: u32,
    order: usize,
}
fn normalize_style_value(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => normalize_style_object(object),
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.iter().map(normalize_style_value).collect())
        }
        value => value.clone(),
    }
}

fn normalize_style_object(
    object: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut normalized = serde_json::Map::new();

    for (key, value) in object {
        if key.starts_with("--") {
            continue;
        }
        let canonical = canonical_property_name(key).unwrap_or(key);
        match canonical {
            "inset" => expand_box_sides(&mut normalized, ["top", "right", "bottom", "left"], value),
            "flex" => {
                if let Some((grow, shrink, basis)) = parse_flex_shorthand_value(value) {
                    normalized.insert("flexGrow".to_string(), serde_json::json!(grow));
                    normalized.insert("flexShrink".to_string(), serde_json::json!(shrink));
                    normalized.insert("flexBasis".to_string(), basis);
                }
            }
            "borderTopWidth" => insert_edge_side(&mut normalized, "borderWidth", "top", value),
            "borderRightWidth" => insert_edge_side(&mut normalized, "borderWidth", "right", value),
            "borderBottomWidth" => {
                insert_edge_side(&mut normalized, "borderWidth", "bottom", value)
            }
            "borderLeftWidth" => insert_edge_side(&mut normalized, "borderWidth", "left", value),
            "transition" => {
                if value.as_str().and_then(parse_transition).is_some() {
                    normalized.insert(canonical.to_string(), normalize_style_value(value));
                }
            }
            _ => {
                normalized.insert(canonical.to_string(), normalize_style_value(value));
            }
        }
    }

    serde_json::Value::Object(normalized)
}

fn expand_box_sides(
    normalized: &mut serde_json::Map<String, serde_json::Value>,
    keys: [&str; 4],
    value: &serde_json::Value,
) {
    let values = edge_shorthand_values(value);
    for (key, value) in keys.into_iter().zip(values) {
        normalized.insert(key.to_string(), normalize_style_value(&value));
    }
}

fn edge_shorthand_values(value: &serde_json::Value) -> [serde_json::Value; 4] {
    if let serde_json::Value::Array(values) = value {
        match values.as_slice() {
            [all] => return [all.clone(), all.clone(), all.clone(), all.clone()],
            [vertical, horizontal] => {
                return [
                    vertical.clone(),
                    horizontal.clone(),
                    vertical.clone(),
                    horizontal.clone(),
                ];
            }
            [top, horizontal, bottom] => {
                return [
                    top.clone(),
                    horizontal.clone(),
                    bottom.clone(),
                    horizontal.clone(),
                ];
            }
            [top, right, bottom, left, ..] => {
                return [top.clone(), right.clone(), bottom.clone(), left.clone()];
            }
            [] => {}
        }
    }

    [value.clone(), value.clone(), value.clone(), value.clone()]
}

fn insert_edge_side(
    normalized: &mut serde_json::Map<String, serde_json::Value>,
    edge_key: &str,
    side: &str,
    value: &serde_json::Value,
) {
    let entry = normalized
        .entry(edge_key.to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    if !entry.is_object() {
        *entry = serde_json::Value::Object(serde_json::Map::new());
    }
    if let Some(object) = entry.as_object_mut() {
        object.insert(side.to_string(), normalize_style_value(value));
    }
}

fn parse_flex_shorthand_value(value: &serde_json::Value) -> Option<(f32, f32, serde_json::Value)> {
    match value {
        serde_json::Value::Number(number) => Some((
            number.as_f64()? as f32,
            1.0,
            serde_json::Value::String("0%".to_string()),
        )),
        serde_json::Value::String(value) => parse_flex_shorthand(value),
        _ => None,
    }
}

fn parse_flex_shorthand(value: &str) -> Option<(f32, f32, serde_json::Value)> {
    let trimmed = value.trim();
    if trimmed.eq_ignore_ascii_case("none") {
        return Some((0.0, 0.0, serde_json::Value::String("auto".to_string())));
    }
    if trimmed.eq_ignore_ascii_case("auto") {
        return Some((1.0, 1.0, serde_json::Value::String("auto".to_string())));
    }
    if let Ok(grow) = trimmed.parse::<f32>() {
        return Some((grow, 1.0, serde_json::Value::String("0%".to_string())));
    }

    let parts = trimmed.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        [grow, basis] => {
            let grow = grow.parse::<f32>().ok()?;
            if let Ok(shrink) = basis.parse::<f32>() {
                Some((grow, shrink, serde_json::Value::String("0%".to_string())))
            } else {
                Some((grow, 1.0, serde_json::Value::String((*basis).to_string())))
            }
        }
        [grow, shrink, basis] => Some((
            grow.parse::<f32>().ok()?,
            shrink.parse::<f32>().ok()?,
            serde_json::Value::String((*basis).to_string()),
        )),
        _ => None,
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
        if property.starts_with("--") {
            continue;
        }
        if property == "placeholder" {
            collect_style_diagnostics(selector, nested, diagnostics);
            continue;
        }

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

        if property == "flex" && parse_flex_shorthand_value(nested).is_none() {
            diagnostics.push(StyleDiagnostic::UnsupportedProperty {
                selector: selector.to_string(),
                property: property.clone(),
            });
            continue;
        }

        if property == "transition" && nested.as_str().and_then(parse_transition).is_none() {
            diagnostics.push(StyleDiagnostic::UnsupportedProperty {
                selector: selector.to_string(),
                property: property.clone(),
            });
            continue;
        }

        if let Some(reason) = unsupported_effect_reason(property) {
            diagnostics.push(StyleDiagnostic::UnsupportedEffect {
                selector: selector.to_string(),
                property: property.clone(),
                reason: reason.to_string(),
            });
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

fn unsupported_effect_reason(property: &str) -> Option<&'static str> {
    match property {
        "borderRadius" | "border-radius" => {
            Some("Bevy UI 0.13 has no native rounded rectangle clipping")
        }
        "boxShadow" | "box-shadow" => {
            Some("box shadows need a custom UI material or extra shadow geometry")
        }
        "filter" => Some("filters need a custom UI material shader"),
        "backdropFilter" | "backdrop-filter" => {
            Some("backdrop filters need a custom render pass/material")
        }
        _ => None,
    }
}

fn matches_state_property(property: &str) -> Option<&str> {
    match property {
        "hover" | "active" | "focus" | "disabled" | "checked" | "selected" | "open" | "valid"
        | "invalid" => Some(property),
        "focusWithin" | "focus-within" => Some("focusWithin"),
        "focusVisible" | "focus-visible" => Some("focusVisible"),
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
        "borderTopWidth" | "border-top-width" => Some("borderTopWidth"),
        "borderRightWidth" | "border-right-width" => Some("borderRightWidth"),
        "borderBottomWidth" | "border-bottom-width" => Some("borderBottomWidth"),
        "borderLeftWidth" | "border-left-width" => Some("borderLeftWidth"),
        "borderColor" | "border-color" => Some("borderColor"),
        "borderTopColor" | "border-top-color" => Some("borderTopColor"),
        "borderRightColor" | "border-right-color" => Some("borderRightColor"),
        "borderBottomColor" | "border-bottom-color" => Some("borderBottomColor"),
        "borderLeftColor" | "border-left-color" => Some("borderLeftColor"),
        "position" => Some("position"),
        "inset" => Some("inset"),
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
        "flex" => Some("flex"),
        "background" | "backgroundColor" | "background-color" => Some("background"),
        "color" => Some("color"),
        "fontSize" | "font-size" => Some("fontSize"),
        "fontFamily" | "font-family" => Some("fontFamily"),
        "fontWeight" | "font-weight" => Some("fontWeight"),
        "fontStyle" | "font-style" => Some("fontStyle"),
        "opacity" => Some("opacity"),
        "outline" => Some("outline"),
        "outlineWidth" | "outline-width" => Some("outlineWidth"),
        "outlineColor" | "outline-color" => Some("outlineColor"),
        "outlineOffset" | "outline-offset" => Some("outlineOffset"),
        "zIndex" | "z-index" => Some("zIndex"),
        "visibility" => Some("visibility"),
        "textAlign" | "text-align" => Some("textAlign"),
        "textWrap" | "text-wrap" => Some("textWrap"),
        "display" => Some("display"),
        "transition" => Some("transition"),
        "placeholder" => Some("placeholder"),
        "borderRadius" | "border-radius" => Some("borderRadius"),
        "boxShadow" | "box-shadow" => Some("boxShadow"),
        "filter" => Some("filter"),
        "backdropFilter" | "backdrop-filter" => Some("backdropFilter"),
        "hover" => Some("hover"),
        "active" => Some("active"),
        "focus" => Some("focus"),
        "checked" => Some("checked"),
        "selected" => Some("selected"),
        "open" => Some("open"),
        "valid" => Some("valid"),
        "invalid" => Some("invalid"),
        "focusWithin" | "focus-within" => Some("focusWithin"),
        "focusVisible" | "focus-visible" => Some("focusVisible"),
        "disabled" => Some("disabled"),
        _ => None,
    }
}

impl StyleSheet {
    pub fn parse(input: &str) -> Result<Self, BevyUiXmlError> {
        Self::parse_with_theme_tokens(input, &HashMap::new())
    }

    pub fn parse_with_theme_tokens(
        input: &str,
        theme_tokens: &HashMap<String, serde_json::Value>,
    ) -> Result<Self, BevyUiXmlError> {
        let value: serde_json::Value = match serde_json::from_str(input) {
            Ok(value) => value,
            Err(json_error) => parse_css_stylesheet(input).unwrap_or_else(|| {
                serde_json::Value::Object({
                    let mut object = serde_json::Map::new();
                    object.insert(
                        "__invalid_css__".to_string(),
                        serde_json::json!({ "__parse_error__": json_error.to_string() }),
                    );
                    object
                })
            }),
        };

        let styles_value = value.get("styles").unwrap_or(&value);
        let Some(styles_object) = styles_value.as_object() else {
            return Ok(Self::default());
        };

        let mut styles = HashMap::new();
        let mut diagnostics = Vec::new();
        let mut rules = Vec::new();
        let root_tokens = collect_root_tokens(styles_object);

        for (order, (selector, style_value)) in styles_object.iter().enumerate() {
            let (selector, style_value) =
                normalize_pseudo_element_style(selector, style_value.clone());
            let local_tokens = collect_custom_properties(&style_value);
            let resolved_style_value = resolve_style_variables(
                &selector,
                &style_value,
                theme_tokens,
                &root_tokens,
                &local_tokens,
                &mut diagnostics,
            );
            collect_style_diagnostics(&selector, &resolved_style_value, &mut diagnostics);
            let style: UiStyle =
                serde_json::from_value(normalize_style_value(&resolved_style_value))?;
            styles.insert(selector.clone(), style.clone());

            let group_members = Selector::parse_group(&selector);
            if group_members.is_empty() {
                diagnostics.push(StyleDiagnostic::InvalidSelector {
                    selector: selector.clone(),
                    reason: "empty selector".to_string(),
                });
                continue;
            }
            for (group_index, member) in group_members.into_iter().enumerate() {
                match member {
                    Ok(selector) => rules.push(StyleRule {
                        specificity: selector.specificity(),
                        order: order * 1024 + group_index,
                        selector,
                        style: style.clone(),
                    }),
                    Err(reason) => diagnostics.push(StyleDiagnostic::InvalidSelector {
                        selector: selector.clone(),
                        reason,
                    }),
                }
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
            self.rule_computed_style(path, None, false)
        }
    }

    pub(crate) fn runtime_base_style_for_path(&self, path: &[&ElementNode]) -> UiStyle {
        if self.rules.is_empty() {
            self.legacy_computed_style(path.last().copied())
        } else {
            self.rule_computed_style(path, None, true)
        }
    }

    pub(crate) fn runtime_state_style_for_path(
        &self,
        path: &[&ElementNode],
        state: PseudoClass,
    ) -> UiStyle {
        if self.rules.is_empty() {
            self.legacy_computed_style(path.last().copied())
        } else {
            self.rule_computed_style(path, Some(state), true)
        }
    }

    pub(crate) fn runtime_ancestor_state_style_for_path(
        &self,
        path: &[&ElementNode],
        state: PseudoClass,
    ) -> UiStyle {
        if self.rules.is_empty() {
            UiStyle::default()
        } else {
            self.rule_computed_ancestor_state_style(path, state)
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

    fn rule_computed_style(
        &self,
        path: &[&ElementNode],
        state: Option<PseudoClass>,
        runtime_state_mode: bool,
    ) -> UiStyle {
        let Some(node) = path.last().copied() else {
            return UiStyle::default();
        };

        let mut matched = self
            .rules
            .iter()
            .filter(|rule| {
                !runtime_state_mode
                    || state.is_none()
                    || state.is_some_and(|state| rule.selector.has_terminal_pseudo(state))
            })
            .filter_map(|rule| {
                rule.selector
                    .matches_with_state(path, state, runtime_state_mode)
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

    fn rule_computed_ancestor_state_style(
        &self,
        path: &[&ElementNode],
        state: PseudoClass,
    ) -> UiStyle {
        let Some(node) = path.last().copied() else {
            return UiStyle::default();
        };

        let mut matched = self
            .rules
            .iter()
            .filter_map(|rule| {
                rule.selector
                    .matches_with_ancestor_state(path, state)
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

fn collect_root_tokens(
    styles_object: &serde_json::Map<String, serde_json::Value>,
) -> HashMap<String, serde_json::Value> {
    let mut tokens = HashMap::new();
    for (selector, value) in styles_object {
        if selector == ":root" || selector == "root" || selector == "ui" {
            tokens.extend(collect_custom_properties(value));
        }
    }
    tokens
}

fn collect_custom_properties(value: &serde_json::Value) -> HashMap<String, serde_json::Value> {
    value
        .as_object()
        .map(|object| {
            object
                .iter()
                .filter(|(key, _)| key.starts_with("--"))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect()
        })
        .unwrap_or_default()
}

fn resolve_style_variables(
    selector: &str,
    value: &serde_json::Value,
    theme_tokens: &HashMap<String, serde_json::Value>,
    root_tokens: &HashMap<String, serde_json::Value>,
    local_tokens: &HashMap<String, serde_json::Value>,
    diagnostics: &mut Vec<StyleDiagnostic>,
) -> serde_json::Value {
    match value {
        serde_json::Value::Object(object) => serde_json::Value::Object(
            object
                .iter()
                .filter(|(key, _)| !key.starts_with("--"))
                .map(|(key, nested)| {
                    (
                        key.clone(),
                        resolve_style_variables(
                            selector,
                            nested,
                            theme_tokens,
                            root_tokens,
                            local_tokens,
                            diagnostics,
                        ),
                    )
                })
                .collect(),
        ),
        serde_json::Value::Array(values) => serde_json::Value::Array(
            values
                .iter()
                .map(|nested| {
                    resolve_style_variables(
                        selector,
                        nested,
                        theme_tokens,
                        root_tokens,
                        local_tokens,
                        diagnostics,
                    )
                })
                .collect(),
        ),
        serde_json::Value::String(text) => resolve_string_variable(
            selector,
            "",
            text,
            theme_tokens,
            root_tokens,
            local_tokens,
            diagnostics,
        )
        .unwrap_or_else(|| value.clone()),
        _ => value.clone(),
    }
}

fn resolve_string_variable(
    selector: &str,
    property: &str,
    text: &str,
    theme_tokens: &HashMap<String, serde_json::Value>,
    root_tokens: &HashMap<String, serde_json::Value>,
    local_tokens: &HashMap<String, serde_json::Value>,
    diagnostics: &mut Vec<StyleDiagnostic>,
) -> Option<serde_json::Value> {
    let trimmed = text.trim();
    let body = trimmed.strip_prefix("var(")?.strip_suffix(')')?;
    let (name, fallback) = body
        .split_once(',')
        .map(|(name, fallback)| (name.trim(), Some(fallback.trim())))
        .unwrap_or((body.trim(), None));
    let resolved = theme_tokens
        .get(name)
        .or_else(|| local_tokens.get(name))
        .or_else(|| root_tokens.get(name))
        .cloned();
    if let Some(value) = resolved {
        return Some(value);
    }
    if let Some(fallback) = fallback {
        return Some(css_value_to_json(fallback));
    }
    diagnostics.push(StyleDiagnostic::UnresolvedVariable {
        selector: selector.to_string(),
        property: property.to_string(),
        variable: name.to_string(),
    });
    None
}

fn normalize_pseudo_element_style(
    selector: &str,
    style_value: serde_json::Value,
) -> (String, serde_json::Value) {
    let Some(base_selector) = selector.strip_suffix("::placeholder") else {
        return (selector.to_string(), style_value);
    };

    let mut wrapped = serde_json::Map::new();
    wrapped.insert("placeholder".to_string(), style_value);
    (
        base_selector.trim().to_string(),
        serde_json::Value::Object(wrapped),
    )
}

fn parse_css_stylesheet(input: &str) -> Option<serde_json::Value> {
    let input = strip_css_comments(input);
    let mut styles = serde_json::Map::new();
    let mut rest = input.as_str();

    while let Some(open) = rest.find('{') {
        let selector = rest[..open].trim();
        let after_open = &rest[open + 1..];
        let close = after_open.find('}')?;
        let body = &after_open[..close];
        rest = &after_open[close + 1..];
        if selector.is_empty() {
            continue;
        }
        styles.insert(selector.to_string(), parse_css_declarations(body));
    }

    (!styles.is_empty()).then_some(serde_json::Value::Object(styles))
}

fn strip_css_comments(input: &str) -> String {
    let mut output = String::new();
    let mut rest = input;
    while let Some(start) = rest.find("/*") {
        output.push_str(&rest[..start]);
        let after_start = &rest[start + 2..];
        if let Some(end) = after_start.find("*/") {
            rest = &after_start[end + 2..];
        } else {
            return output;
        }
    }
    output.push_str(rest);
    output
}

fn parse_css_declarations(body: &str) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    for declaration in split_css_declarations(body) {
        let Some((name, value)) = declaration.split_once(':') else {
            continue;
        };
        object.insert(
            name.trim().to_string(),
            css_value_to_json(value.trim().trim_end_matches(';').trim()),
        );
    }
    serde_json::Value::Object(object)
}

fn split_css_declarations(body: &str) -> Vec<String> {
    let mut declarations = Vec::new();
    let mut depth = 0usize;
    let mut quote = None;
    let mut start = 0usize;

    for (index, ch) in body.char_indices() {
        match ch {
            '"' | '\'' => {
                if quote == Some(ch) {
                    quote = None;
                } else if quote.is_none() {
                    quote = Some(ch);
                }
            }
            '(' if quote.is_none() => depth += 1,
            ')' if quote.is_none() => depth = depth.saturating_sub(1),
            ';' if quote.is_none() && depth == 0 => {
                declarations.push(body[start..index].trim().to_string());
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }

    let tail = body[start..].trim();
    if !tail.is_empty() {
        declarations.push(tail.to_string());
    }
    declarations
}

fn css_value_to_json(value: &str) -> serde_json::Value {
    let unquoted = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        });
    if let Some(value) = unquoted {
        return serde_json::Value::String(value.to_string());
    }
    if let Ok(number) = value.parse::<f64>() {
        return serde_json::json!(number);
    }
    serde_json::Value::String(value.to_string())
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
    pub border_top_color: Option<String>,
    pub border_right_color: Option<String>,
    pub border_bottom_color: Option<String>,
    pub border_left_color: Option<String>,
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
    pub font_family: Option<serde_json::Value>,
    pub font_weight: Option<serde_json::Value>,
    pub font_style: Option<serde_json::Value>,
    pub opacity: Option<f32>,
    pub outline: Option<OutlineStyle>,
    pub outline_width: Option<Length>,
    pub outline_color: Option<String>,
    pub outline_offset: Option<Length>,
    pub z_index: Option<i32>,
    pub visibility: Option<VisibilityValue>,
    pub text_align: Option<TextAlignValue>,
    pub text_wrap: Option<TextWrapValue>,
    pub border_radius: Option<String>,
    pub box_shadow: Option<String>,
    pub filter: Option<String>,
    pub backdrop_filter: Option<String>,
    pub display: Option<DisplayValue>,
    pub transition: Option<TransitionStyle>,
    pub placeholder: Option<Box<UiStyle>>,
    pub hover: Option<Box<UiStyle>>,
    pub active: Option<Box<UiStyle>>,
    pub focus: Option<Box<UiStyle>>,
    pub checked: Option<Box<UiStyle>>,
    pub selected: Option<Box<UiStyle>>,
    pub open: Option<Box<UiStyle>>,
    pub valid: Option<Box<UiStyle>>,
    pub invalid: Option<Box<UiStyle>>,
    pub focus_within: Option<Box<UiStyle>>,
    pub focus_visible: Option<Box<UiStyle>>,
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
        merge_field!(border_top_color);
        merge_field!(border_right_color);
        merge_field!(border_bottom_color);
        merge_field!(border_left_color);
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
        merge_field!(font_family);
        merge_field!(font_weight);
        merge_field!(font_style);
        merge_field!(opacity);
        merge_field!(outline);
        merge_field!(outline_width);
        merge_field!(outline_color);
        merge_field!(outline_offset);
        merge_field!(z_index);
        merge_field!(visibility);
        merge_field!(text_align);
        merge_field!(text_wrap);
        merge_field!(border_radius);
        merge_field!(box_shadow);
        merge_field!(filter);
        merge_field!(backdrop_filter);
        merge_field!(display);
        merge_field!(transition);
        merge_field!(placeholder);
        merge_field!(hover);
        merge_field!(active);
        merge_field!(focus);
        merge_field!(checked);
        merge_field!(selected);
        merge_field!(open);
        merge_field!(valid);
        merge_field!(invalid);
        merge_field!(focus_within);
        merge_field!(focus_visible);
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

    pub(crate) fn without_state_styles(&self) -> Self {
        let mut style = self.clone();
        style.hover = None;
        style.active = None;
        style.focus = None;
        style.checked = None;
        style.selected = None;
        style.open = None;
        style.valid = None;
        style.invalid = None;
        style.focus_within = None;
        style.focus_visible = None;
        style.disabled = None;
        style.placeholder = None;
        style
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub struct TransitionStyle {
    pub property: TransitionProperty,
    pub duration: f32,
    pub easing: TransitionEasing,
}

impl TryFrom<String> for TransitionStyle {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        parse_transition(&value).ok_or_else(|| "unsupported transition".to_string())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionProperty {
    Background,
    Color,
    Opacity,
    OutlineColor,
    OutlineWidth,
    BorderColor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionEasing {
    Linear,
    Ease,
    EaseIn,
    EaseOut,
}

fn parse_transition(value: &str) -> Option<TransitionStyle> {
    let mut property = None;
    let mut duration = None;
    let mut easing = TransitionEasing::Linear;
    for part in value.split_whitespace() {
        if property.is_none() {
            property = parse_transition_property(part);
            if property.is_some() {
                continue;
            }
        }
        if duration.is_none() {
            duration = parse_duration(part);
            if duration.is_some() {
                continue;
            }
        }
        if let Some(parsed) = parse_transition_easing(part) {
            easing = parsed;
        }
    }

    Some(TransitionStyle {
        property: property?,
        duration: duration?,
        easing,
    })
}

fn parse_transition_property(value: &str) -> Option<TransitionProperty> {
    match value {
        "background" | "backgroundColor" | "background-color" => {
            Some(TransitionProperty::Background)
        }
        "color" => Some(TransitionProperty::Color),
        "opacity" => Some(TransitionProperty::Opacity),
        "outlineColor" | "outline-color" => Some(TransitionProperty::OutlineColor),
        "outlineWidth" | "outline-width" => Some(TransitionProperty::OutlineWidth),
        "borderColor" | "border-color" => Some(TransitionProperty::BorderColor),
        _ => None,
    }
}

fn parse_transition_easing(value: &str) -> Option<TransitionEasing> {
    match value {
        "linear" => Some(TransitionEasing::Linear),
        "ease" => Some(TransitionEasing::Ease),
        "ease-in" => Some(TransitionEasing::EaseIn),
        "ease-out" => Some(TransitionEasing::EaseOut),
        _ => None,
    }
}

fn parse_duration(value: &str) -> Option<f32> {
    value
        .strip_suffix("ms")
        .and_then(|ms| ms.parse::<f32>().ok())
        .map(|ms| ms / 1000.0)
        .or_else(|| {
            value
                .strip_suffix('s')
                .and_then(|seconds| seconds.parse::<f32>().ok())
        })
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

    pub(crate) fn into_val(self) -> Val {
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

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum OutlineStyle {
    Color(String),
    Parts {
        width: Option<Length>,
        color: Option<String>,
        offset: Option<Length>,
    },
}

impl OutlineStyle {
    pub(crate) fn width(&self) -> Option<Length> {
        match self {
            Self::Color(_) => None,
            Self::Parts { width, .. } => width.clone(),
        }
    }

    pub(crate) fn color(&self) -> Option<&str> {
        match self {
            Self::Color(color) => Some(color),
            Self::Parts { color, .. } => color.as_deref(),
        }
    }

    pub(crate) fn offset(&self) -> Option<Length> {
        match self {
            Self::Color(_) => None,
            Self::Parts { offset, .. } => offset.clone(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisibilityValue {
    Visible,
    Hidden,
    Inherited,
}

impl VisibilityValue {
    pub(crate) fn to_bevy(self) -> Visibility {
        match self {
            Self::Visible => Visibility::Visible,
            Self::Hidden => Visibility::Hidden,
            Self::Inherited => Visibility::Inherited,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextAlignValue {
    Left,
    Center,
    Right,
}

impl TextAlignValue {
    pub(crate) fn to_bevy(self) -> JustifyText {
        match self {
            Self::Left => JustifyText::Left,
            Self::Center => JustifyText::Center,
            Self::Right => JustifyText::Right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TextWrapValue {
    Normal,
    WordBoundary,
    AnyCharacter,
    None,
    NoWrap,
}

impl TextWrapValue {
    pub(crate) fn to_bevy(self) -> BreakLineOn {
        match self {
            Self::Normal | Self::WordBoundary => BreakLineOn::WordBoundary,
            Self::AnyCharacter => BreakLineOn::AnyCharacter,
            Self::None | Self::NoWrap => BreakLineOn::NoWrap,
        }
    }
}
pub(crate) fn to_bevy_style(style: &UiStyle) -> Style {
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

pub(crate) fn style_color(value: Option<&str>, fallback: Color, opacity: Option<f32>) -> Color {
    let mut color = parse_color(value).unwrap_or(fallback);
    if let Some(opacity) = opacity {
        let [r, g, b, a] = color.as_rgba_f32();
        color = Color::rgba(r, g, b, a * opacity.clamp(0.0, 1.0));
    }
    color
}

pub(crate) fn parse_color(value: Option<&str>) -> Option<Color> {
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
        4 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).ok()?;
            Some(Color::rgba_u8(r, g, b, a))
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
    let parts = split_rgb_args(args);
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

fn split_rgb_args(value: &str) -> Vec<String> {
    let comma_parts = split_css_args(value);
    if comma_parts.len() > 1 {
        return comma_parts;
    }

    let mut parts = Vec::new();
    let mut alpha = None;
    let mut before_slash = value;
    if let Some((channels, alpha_value)) = value.split_once('/') {
        before_slash = channels;
        alpha = Some(alpha_value.trim().to_string());
    }
    parts.extend(before_slash.split_whitespace().map(str::to_string));
    if let Some(alpha) = alpha {
        parts.push(alpha);
    }
    parts
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

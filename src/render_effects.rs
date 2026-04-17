use crate::style::{style_color, Length, OutlineStyle, UiStyle};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, PartialEq)]
pub struct UiXmlUnsupportedEffects {
    pub effects: Vec<UnsupportedEffect>,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct UiXmlBorderColors {
    pub top: Option<Color>,
    pub right: Option<Color>,
    pub bottom: Option<Color>,
    pub left: Option<Color>,
}

#[derive(Component, Debug, Clone, PartialEq)]
pub struct UiXmlRenderMaterialSpec {
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub border_radius: Option<String>,
    pub box_shadow: Option<String>,
    pub filter: Option<String>,
    pub backdrop_filter: Option<String>,
    pub gradient: Option<String>,
    pub gradient_end: Option<Color>,
}

impl UiXmlRenderMaterialSpec {
    pub(crate) fn radius_strength(&self) -> f32 {
        self.border_radius
            .as_deref()
            .and_then(|value| value.trim_end_matches("px").trim().parse::<f32>().ok())
            .map(|value| (value / 64.0).clamp(0.0, 0.5))
            .unwrap_or(0.0)
    }

    pub(crate) fn shadow_alpha(&self) -> f32 {
        self.box_shadow
            .as_ref()
            .map(|_| 0.25)
            .or_else(|| self.filter.as_ref().map(|_| 0.1))
            .unwrap_or(0.0)
    }

    pub(crate) fn border_width_strength(&self) -> f32 {
        self.border_color.map(|_| 0.025).unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnsupportedEffect {
    BorderRadius(String),
    BoxShadow(String),
    Filter(String),
    BackdropFilter(String),
}
pub(crate) fn outline_from_style(style: &UiStyle) -> Option<Outline> {
    let width = style
        .outline_width
        .clone()
        .or_else(|| style.outline.as_ref().and_then(OutlineStyle::width))
        .map(Length::into_val)?;
    let offset = style
        .outline_offset
        .clone()
        .or_else(|| style.outline.as_ref().and_then(OutlineStyle::offset))
        .map(Length::into_val)
        .unwrap_or(Val::ZERO);
    let color = style_color(
        style
            .outline_color
            .as_deref()
            .or_else(|| style.outline.as_ref().and_then(OutlineStyle::color)),
        Color::WHITE,
        style.opacity,
    );

    Some(Outline::new(width, offset, color))
}

pub(crate) fn unsupported_effects_from_style(style: &UiStyle) -> Option<UiXmlUnsupportedEffects> {
    let mut effects = Vec::new();
    if let Some(value) = &style.border_radius {
        effects.push(UnsupportedEffect::BorderRadius(value.clone()));
    }
    if let Some(value) = &style.box_shadow {
        effects.push(UnsupportedEffect::BoxShadow(value.clone()));
    }
    if let Some(value) = &style.filter {
        effects.push(UnsupportedEffect::Filter(value.clone()));
    }
    if let Some(value) = &style.backdrop_filter {
        effects.push(UnsupportedEffect::BackdropFilter(value.clone()));
    }

    (!effects.is_empty()).then_some(UiXmlUnsupportedEffects { effects })
}

pub(crate) fn border_colors_from_style(style: &UiStyle) -> Option<UiXmlBorderColors> {
    let colors = UiXmlBorderColors {
        top: style
            .border_top_color
            .as_deref()
            .and_then(|color| crate::style::parse_color(Some(color))),
        right: style
            .border_right_color
            .as_deref()
            .and_then(|color| crate::style::parse_color(Some(color))),
        bottom: style
            .border_bottom_color
            .as_deref()
            .and_then(|color| crate::style::parse_color(Some(color))),
        left: style
            .border_left_color
            .as_deref()
            .and_then(|color| crate::style::parse_color(Some(color))),
    };

    (colors.top.is_some()
        || colors.right.is_some()
        || colors.bottom.is_some()
        || colors.left.is_some())
    .then_some(colors)
}

pub(crate) fn render_material_spec_from_style(style: &UiStyle) -> Option<UiXmlRenderMaterialSpec> {
    let gradient = style
        .background
        .as_deref()
        .filter(|value| value.contains("gradient"))
        .map(str::to_string);
    let gradient_end = gradient.as_deref().and_then(parse_gradient_end_color);
    let spec = UiXmlRenderMaterialSpec {
        background: style
            .background
            .as_deref()
            .and_then(|color| crate::style::parse_color(Some(color))),
        border_color: style
            .border_color
            .as_deref()
            .or(style.border_top_color.as_deref())
            .and_then(|color| crate::style::parse_color(Some(color))),
        border_radius: style.border_radius.clone(),
        box_shadow: style.box_shadow.clone(),
        filter: style.filter.clone(),
        backdrop_filter: style.backdrop_filter.clone(),
        gradient,
        gradient_end,
    };

    (spec.border_radius.is_some()
        || spec.box_shadow.is_some()
        || spec.filter.is_some()
        || spec.backdrop_filter.is_some()
        || spec.gradient.is_some()
        || spec.border_color.is_some())
    .then_some(spec)
}

fn parse_gradient_end_color(value: &str) -> Option<Color> {
    let inside = value.split_once('(')?.1.strip_suffix(')')?;
    inside.rsplit(',').find_map(|part| {
        part.split_whitespace()
            .next()
            .and_then(|token| crate::style::parse_color(Some(token)))
    })
}

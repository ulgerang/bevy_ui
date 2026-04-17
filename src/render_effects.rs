use crate::style::{style_color, Length, OutlineStyle, UiStyle};
use bevy::prelude::*;

#[derive(Component, Debug, Clone, PartialEq)]
pub struct UiXmlUnsupportedEffects {
    pub effects: Vec<UnsupportedEffect>,
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

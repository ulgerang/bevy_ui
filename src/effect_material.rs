use bevy::asset::AssetApp;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::ui::{UiMaterial, UiMaterialPlugin};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, PartialEq)]
pub struct UiXmlEffectMaterial {
    #[uniform(0)]
    pub color: Color,
    #[uniform(0)]
    pub border_color: Color,
    #[uniform(0)]
    pub gradient_end: Color,
    #[uniform(0)]
    pub radius: f32,
    #[uniform(0)]
    pub border_width: f32,
    #[uniform(0)]
    pub gradient_mix: f32,
    #[uniform(0)]
    pub shadow_alpha: f32,
}

impl Default for UiXmlEffectMaterial {
    fn default() -> Self {
        Self {
            color: Color::NONE,
            border_color: Color::NONE,
            gradient_end: Color::NONE,
            radius: 0.0,
            border_width: 0.0,
            gradient_mix: 0.0,
            shadow_alpha: 0.0,
        }
    }
}

impl UiMaterial for UiXmlEffectMaterial {
    fn fragment_shader() -> ShaderRef {
        "uixml_effect_material.wgsl".into()
    }
}

#[derive(Default)]
pub struct UiXmlEffectMaterialPlugin;

impl Plugin for UiXmlEffectMaterialPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<UiXmlEffectMaterial>()
            .add_plugins(UiMaterialPlugin::<UiXmlEffectMaterial>::default());
    }
}

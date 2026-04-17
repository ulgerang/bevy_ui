use bevy::asset::AssetApp;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderRef};
use bevy::ui::{UiMaterial, UiMaterialPlugin};

#[derive(AsBindGroup, Asset, TypePath, Debug, Clone, PartialEq)]
pub struct UiXmlEffectMaterial {
    #[uniform(0)]
    pub color: Color,
    #[uniform(0)]
    pub radius: f32,
    #[uniform(0)]
    pub shadow_alpha: f32,
}

impl Default for UiXmlEffectMaterial {
    fn default() -> Self {
        Self {
            color: Color::NONE,
            radius: 0.0,
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

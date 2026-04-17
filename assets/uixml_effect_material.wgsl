#import bevy_ui::ui_vertex_output::UiVertexOutput

struct UiXmlEffectMaterial {
    color: vec4<f32>,
    radius: f32,
    shadow_alpha: f32,
    _pad0: vec2<f32>,
};

@group(1) @binding(0)
var<uniform> material: UiXmlEffectMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let edge_distance = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let rounded_alpha = smoothstep(0.0, max(material.radius, 0.0001), edge_distance + material.radius * 0.5);
    let shadow = material.shadow_alpha * (1.0 - smoothstep(0.0, 0.35, edge_distance));
    let rgb = material.color.rgb * (1.0 - shadow);
    return vec4<f32>(rgb, material.color.a * rounded_alpha);
}

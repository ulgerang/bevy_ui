#import bevy_ui::ui_vertex_output::UiVertexOutput

struct UiXmlEffectMaterial {
    color: vec4<f32>,
    border_color: vec4<f32>,
    gradient_end: vec4<f32>,
    radius: f32,
    border_width: f32,
    gradient_mix: f32,
    shadow_alpha: f32,
};

@group(1) @binding(0)
var<uniform> material: UiXmlEffectMaterial;

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let edge_distance = min(min(uv.x, 1.0 - uv.x), min(uv.y, 1.0 - uv.y));
    let rounded_alpha = smoothstep(0.0, max(material.radius, 0.0001), edge_distance + material.radius * 0.5);
    let gradient_color = mix(material.color, material.gradient_end, clamp(uv.x * material.gradient_mix, 0.0, 1.0));
    let border_factor = 1.0 - smoothstep(0.0, max(material.border_width, 0.0001), edge_distance);
    let panel_color = mix(gradient_color, material.border_color, border_factor * material.border_color.a);
    let shadow = material.shadow_alpha * (1.0 - smoothstep(0.0, 0.35, edge_distance));
    let rgb = panel_color.rgb * (1.0 - shadow);
    return vec4<f32>(rgb, panel_color.a * rounded_alpha);
}

# ADR: Render Effects Boundary

Status: Accepted for Stage 10.

## Decision

`borderRadius`, `boxShadow`, `filter`, and `backdropFilter` remain unsupported render effects. They are preserved as spawn-time `UiXmlUnsupportedEffects` metadata for diagnostics and future renderer work.

Gradients are not rendered as gradients in this track; color parsing may use a
deterministic first-stop fallback only. Side-specific border colors such as
`borderTopColor` are diagnostic-only because Bevy UI 0.13 exposes one runtime
`BorderColor` for the entity.

Runtime hover/active/disabled restyling updates Bevy-supported UI properties such as `Style`, colors, and `Outline`. It does not update or remove `UiXmlUnsupportedEffects`.

## Drivers

- Bevy UI 0.13.2 does not provide browser-compatible rendering for these effects.
- Custom material or render pipeline work would be larger than this form-control slice.
- Public docs must not imply visual support for metadata-only properties.

## Alternatives Considered

- Continue metadata-only: chosen.
- Map a native subset now: deferred until a smaller supported subset is identified.
- Map side-specific border colors onto the single `BorderColor`: rejected
  because it would silently misrepresent author intent.
- Add a custom UI renderer: rejected for this stage.
- Wait for a Bevy upgrade: possible future path.

## Consequences

- Unsupported effect values remain inspectable.
- State-specific unsupported effects are not live render components.
- Gradient strings do not imply renderer support.
- Side-specific border color declarations do not affect runtime `BorderColor`.
- Any future visual support requires a renderer ADR and tests.

## Renderer Handoff

`UiXmlRenderMaterialSpec` captures resolved background/effect inputs.
`UiXmlEffectMaterial` and `UiXmlEffectMaterialPlugin` provide an opt-in Bevy
`UiMaterial` shader path for projects that include Bevy render plugins. The
default `UiXmlPlugin` does not install the render plugin so headless tests and
non-render apps remain lightweight. The bundled WGSL shader is a bounded first
pass for tinting, approximate rounded alpha, simple border color, horizontal
two-stop gradient blending, and shadow edge darkening; it does not claim
browser-equivalent filters, backdrop filters, arbitrary gradients, or
layout-affecting shadows.

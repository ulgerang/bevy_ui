# ADR: Asset Loading Boundary

Status: Accepted for asset loading and hot reload slice.

## Decision

The crate keeps the existing string-first APIs and adds bounded Bevy AssetServer
integration for game iteration workflows.

Supported now:

- `UiXmlLayoutAsset` for parsed XML layout documents.
- `UiXmlStyleAsset` for parsed JSON/native CSS style assets.
- `UiXmlLayoutAssetLoader` and `UiXmlStyleAssetLoader` registered by the opt-in
  `UiXmlAssetPlugin`.
- `UiXmlAssetDocument` for asset-backed UI roots using layout/style handles.
- Asset-backed roots spawn when both handles are loaded and rebuild their child
  UI when matching layout/style assets are added, modified, or fully loaded.
- `UiXmlStyleRuntime.generation` increments on style asset events and active
  `UiXmlThemeTokens` changes.
- `UiXmlAssetDiagnostic` carries source path plus diagnostic message.

`UiXmlPlugin` remains headless-safe. Projects that want AssetServer loading add
`AssetPlugin` plus `UiXmlAssetPlugin`.

## Drivers

- Game UI iteration benefits from editing XML/CSS assets instead of recompiling
  string constants.
- Bevy already owns asset identity, loading state, handles, and hot reload
  events.
- Existing string APIs are useful for tests and embedded UI definitions and must
  remain stable.

## Alternatives Considered

- String-only forever: rejected because it slows game iteration.
- Full document dependency graph/import system: rejected as too broad for this
  slice.
- Register asset loaders in `UiXmlPlugin`: rejected because headless/minimal apps
  may not have AssetServer resources.

## Consequences

- Asset workflow is opt-in through `UiXmlAssetPlugin`.
- Style reload uses generation/rebuild semantics rather than full incremental
  CSSOM diffing.
- Layout reload currently rebuilds the asset-backed root's child UI.
- CSS imports and complex dependency tracking remain deferred.

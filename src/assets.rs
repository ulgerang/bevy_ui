use crate::{parse_layout, BevyUiXmlError, StyleDiagnostic, StyleSheet, UiDocument};
use bevy::asset::{
    io::Reader, AssetEvent, AssetId, AssetLoader, AsyncReadExt, BoxedFuture, LoadContext,
};
use bevy::prelude::*;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiXmlAssetDiagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Asset, TypePath, Debug, Clone)]
pub struct UiXmlLayoutAsset {
    pub document: UiDocument,
    pub source_path: String,
    pub diagnostics: Vec<UiXmlAssetDiagnostic>,
}

#[derive(Asset, TypePath, Debug, Clone)]
pub struct UiXmlStyleAsset {
    pub stylesheet: StyleSheet,
    pub source_path: String,
    pub diagnostics: Vec<UiXmlAssetDiagnostic>,
}

#[derive(Default)]
pub struct UiXmlLayoutAssetLoader;

#[derive(Default)]
pub struct UiXmlStyleAssetLoader;

#[derive(Debug, Error)]
pub enum UiXmlAssetLoadError {
    #[error("I/O error while loading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse XML layout {path}: {source}")]
    LayoutParse {
        path: String,
        #[source]
        source: BevyUiXmlError,
    },
    #[error("failed to parse style asset {path}: {source}")]
    StyleParse {
        path: String,
        #[source]
        source: BevyUiXmlError,
    },
}

impl AssetLoader for UiXmlLayoutAssetLoader {
    type Asset = UiXmlLayoutAsset;
    type Settings = ();
    type Error = UiXmlAssetLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let path = load_context.path().display().to_string();
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(|source| UiXmlAssetLoadError::Io {
                    path: path.clone(),
                    source,
                })?;
            let text = String::from_utf8_lossy(&bytes);
            let document =
                parse_layout(&text).map_err(|source| UiXmlAssetLoadError::LayoutParse {
                    path: path.clone(),
                    source,
                })?;
            Ok(UiXmlLayoutAsset {
                document,
                source_path: path,
                diagnostics: Vec::new(),
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["xml", "uixml"]
    }
}

impl AssetLoader for UiXmlStyleAssetLoader {
    type Asset = UiXmlStyleAsset;
    type Settings = ();
    type Error = UiXmlAssetLoadError;

    fn load<'a>(
        &'a self,
        reader: &'a mut Reader,
        _settings: &'a Self::Settings,
        load_context: &'a mut LoadContext,
    ) -> BoxedFuture<'a, Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let path = load_context.path().display().to_string();
            let mut bytes = Vec::new();
            reader
                .read_to_end(&mut bytes)
                .await
                .map_err(|source| UiXmlAssetLoadError::Io {
                    path: path.clone(),
                    source,
                })?;
            let text = String::from_utf8_lossy(&bytes);
            let stylesheet =
                StyleSheet::parse(&text).map_err(|source| UiXmlAssetLoadError::StyleParse {
                    path: path.clone(),
                    source,
                })?;
            let diagnostics = stylesheet
                .diagnostics
                .iter()
                .map(|diagnostic| UiXmlAssetDiagnostic {
                    path: path.clone(),
                    message: style_diagnostic_message(diagnostic),
                })
                .collect();
            Ok(UiXmlStyleAsset {
                stylesheet,
                source_path: path,
                diagnostics,
            })
        })
    }

    fn extensions(&self) -> &[&str] {
        &["css", "json"]
    }
}

fn style_diagnostic_message(diagnostic: &StyleDiagnostic) -> String {
    match diagnostic {
        StyleDiagnostic::UnsupportedProperty { selector, property } => {
            format!("unsupported property `{property}` in `{selector}`")
        }
        StyleDiagnostic::UnsupportedEffect {
            selector,
            property,
            reason,
        } => format!("unsupported effect `{property}` in `{selector}`: {reason}"),
        StyleDiagnostic::InvalidSelector { selector, reason } => {
            format!("invalid selector `{selector}`: {reason}")
        }
        StyleDiagnostic::UnresolvedVariable {
            selector,
            property,
            variable,
        } => format!("unresolved variable `{variable}` for `{property}` in `{selector}`"),
    }
}

#[derive(Component, Debug, Clone)]
pub struct UiXmlAssetDocument {
    pub layout: Handle<UiXmlLayoutAsset>,
    pub style: Handle<UiXmlStyleAsset>,
    pub default_font: Option<String>,
    pub spawned_root: Option<Entity>,
}

impl UiXmlAssetDocument {
    pub fn new(layout: Handle<UiXmlLayoutAsset>, style: Handle<UiXmlStyleAsset>) -> Self {
        Self {
            layout,
            style,
            default_font: None,
            spawned_root: None,
        }
    }

    pub fn with_default_font(mut self, path: impl Into<String>) -> Self {
        self.default_font = Some(path.into());
        self
    }
}

#[derive(Default)]
pub struct UiXmlAssetPlugin;

impl Plugin for UiXmlAssetPlugin {
    fn build(&self, app: &mut App) {
        use bevy::asset::AssetApp;

        app.init_asset::<UiXmlLayoutAsset>()
            .init_asset::<UiXmlStyleAsset>()
            .register_asset_loader(UiXmlLayoutAssetLoader)
            .register_asset_loader(UiXmlStyleAssetLoader)
            .add_systems(
                Update,
                (
                    mark_style_runtime_for_asset_events,
                    mark_asset_documents_dirty,
                    spawn_ready_asset_documents,
                )
                    .chain(),
            );
    }
}

pub fn spawn_asset_document(
    commands: &mut Commands<'_, '_>,
    layout: Handle<UiXmlLayoutAsset>,
    style: Handle<UiXmlStyleAsset>,
) -> Entity {
    commands.spawn(UiXmlAssetDocument::new(layout, style)).id()
}

fn mark_style_runtime_for_asset_events(
    mut events: EventReader<AssetEvent<UiXmlStyleAsset>>,
    mut runtime: ResMut<crate::runtime::UiXmlStyleRuntime>,
) {
    if events.read().any(|event| {
        matches!(
            event,
            AssetEvent::Added { .. }
                | AssetEvent::Modified { .. }
                | AssetEvent::LoadedWithDependencies { .. }
        )
    }) {
        runtime.generation += 1;
    }
}

fn mark_asset_documents_dirty(
    mut commands: Commands<'_, '_>,
    mut layout_events: EventReader<AssetEvent<UiXmlLayoutAsset>>,
    mut style_events: EventReader<AssetEvent<UiXmlStyleAsset>>,
    mut documents: Query<&mut UiXmlAssetDocument>,
) {
    let changed_layouts = layout_events
        .read()
        .filter_map(layout_event_id)
        .collect::<Vec<_>>();
    let changed_styles = style_events
        .read()
        .filter_map(style_event_id)
        .collect::<Vec<_>>();

    if changed_layouts.is_empty() && changed_styles.is_empty() {
        return;
    }

    for mut document in &mut documents {
        let layout_changed = changed_layouts.iter().any(|id| *id == document.layout.id());
        let style_changed = changed_styles.iter().any(|id| *id == document.style.id());
        if !layout_changed && !style_changed {
            continue;
        }
        if let Some(root) = document.spawned_root.take() {
            commands.entity(root).despawn_recursive();
        }
    }
}

fn layout_event_id(event: &AssetEvent<UiXmlLayoutAsset>) -> Option<AssetId<UiXmlLayoutAsset>> {
    match event {
        AssetEvent::Added { id }
        | AssetEvent::Modified { id }
        | AssetEvent::LoadedWithDependencies { id } => Some(*id),
        AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => None,
    }
}

fn style_event_id(event: &AssetEvent<UiXmlStyleAsset>) -> Option<AssetId<UiXmlStyleAsset>> {
    match event {
        AssetEvent::Added { id }
        | AssetEvent::Modified { id }
        | AssetEvent::LoadedWithDependencies { id } => Some(*id),
        AssetEvent::Removed { .. } | AssetEvent::Unused { .. } => None,
    }
}

fn spawn_ready_asset_documents(
    mut commands: Commands<'_, '_>,
    asset_server: Res<AssetServer>,
    layouts: Res<Assets<UiXmlLayoutAsset>>,
    styles: Res<Assets<UiXmlStyleAsset>>,
    mut documents: Query<(Entity, &mut UiXmlAssetDocument)>,
) {
    for (entity, mut document) in &mut documents {
        if document.spawned_root.is_some() {
            continue;
        }
        let Some(layout) = layouts.get(&document.layout) else {
            continue;
        };
        let Some(style) = styles.get(&document.style) else {
            continue;
        };
        let root = if let Some(default_font) = document.default_font.as_deref() {
            crate::spawn_document(
                &mut commands,
                &asset_server,
                &layout.document,
                &style.stylesheet,
                default_font,
            )
        } else {
            crate::spawn_document_with_embedded_font(
                &mut commands,
                &asset_server,
                &layout.document,
                &style.stylesheet,
            )
        };
        commands.entity(entity).add_child(root);
        document.spawned_root = Some(root);
    }
}

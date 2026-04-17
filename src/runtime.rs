use crate::render_effects::{
    border_colors_from_style, outline_from_style, render_material_spec_from_style,
    unsupported_effects_from_style, UiXmlRenderMaterialSpec,
};
use crate::style::{style_color, to_bevy_style, TransitionProperty, UiStyle};
use crate::{ElementNode, UiXmlEffectMaterial};
use bevy::ecs::system::SystemParam;
use bevy::input::gamepad::{
    GamepadAxisChangedEvent, GamepadAxisType, GamepadButtonInput, GamepadButtonType,
};
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::{Ime, ReceivedCharacter};
use std::collections::{HashMap, HashSet};

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlElement {
    pub tag: String,
    pub widget_type: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: HashMap<String, String>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlDisabled(pub bool);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiXmlControlKind {
    Checkbox,
    Radio,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlChecked(pub bool);

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlControlValue(pub String);

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlControlName(pub String);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiXmlControlScope(pub Entity);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiXmlDocumentOrder(pub usize);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlForm;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlRequired(pub bool);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlSelected(pub bool);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlOpen(pub bool);

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlValidationState {
    pub valid: bool,
    pub reason: Option<String>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlInitialChecked(pub bool);

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlInitialTextValue(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiXmlFormValue {
    pub name: String,
    pub value: String,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlFormSubmitRequested {
    pub form: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlFormResetRequested {
    pub form: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlFormSubmitted {
    pub form: Entity,
    pub values: Vec<UiXmlFormValue>,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlFormValidationFailed {
    pub form: Entity,
    pub entity: Entity,
    pub name: Option<String>,
    pub reason: String,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlValidationStateChanged {
    pub entity: Entity,
    pub valid: bool,
    pub reason: Option<String>,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlNavigationRequested {
    pub form: Entity,
    pub action: Option<String>,
    pub method: Option<String>,
    pub values: Vec<UiXmlFormValue>,
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlFocusable {
    pub order: usize,
    pub tab_index: Option<i32>,
    pub up: Option<String>,
    pub down: Option<String>,
    pub left: Option<String>,
    pub right: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiXmlNavigationDirection {
    Next,
    Previous,
    Up,
    Down,
    Left,
    Right,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlFocusChanged {
    pub previous: Option<Entity>,
    pub current: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlActivateRequested {
    pub entity: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlBackRequested {
    pub focused: Option<Entity>,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlControlChanged {
    pub entity: Entity,
    pub kind: UiXmlControlKind,
    pub scope: Entity,
    pub name: Option<String>,
    pub value: String,
    pub checked: bool,
    pub previous_checked: bool,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlTextArea;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlTextInput;

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlTextValue(pub String);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlTextCursor {
    pub position: usize,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlTextSelection {
    pub anchor: usize,
    pub focus: usize,
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlImePreedit {
    pub value: String,
    pub cursor: Option<(usize, usize)>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiXmlTextDisplay(pub Entity);

#[derive(Component, Debug, Clone, PartialEq)]
pub struct UiXmlTextPlaceholder {
    pub text: String,
    pub placeholder_color: Option<Color>,
    pub placeholder_font_size: Option<f32>,
    pub value_color: Color,
    pub value_font_size: f32,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlTextChanged {
    pub entity: Entity,
    pub scope: Entity,
    pub name: Option<String>,
    pub previous_value: String,
    pub value: String,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlSelect;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlOption {
    pub value: String,
}

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlSelectValue(pub String);

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlSelectChanged {
    pub select: Entity,
    pub option: Entity,
    pub previous_value: String,
    pub value: String,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlRange {
    pub min: f32,
    pub max: f32,
    pub step: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlRangeValue(pub f32);

#[derive(Event, Debug, Clone, PartialEq)]
pub struct UiXmlRangeChanged {
    pub entity: Entity,
    pub previous_value: f32,
    pub value: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlProgress {
    pub value: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlMeter {
    pub value: f32,
    pub min: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlFillPercent(pub f32);

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlScrollContainer {
    pub min: f32,
    pub max: f32,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct UiXmlScrollOffset(pub f32);

#[derive(Event, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlScrollRequested {
    pub entity: Entity,
    pub delta: f32,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlClipboardCopyRequested {
    pub entity: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlClipboardCutRequested {
    pub entity: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlClipboardPasteRequested {
    pub entity: Entity,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlTextSelectAllRequested {
    pub entity: Entity,
}

#[derive(Resource, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlClipboard {
    pub text: String,
}

#[derive(Resource, Debug, Clone, PartialEq, Default)]
pub struct UiXmlThemeTokens {
    pub tokens: HashMap<String, serde_json::Value>,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlFocus {
    pub entity: Option<Entity>,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiXmlInputModality {
    pub focus_visible: bool,
}

impl Default for UiXmlInputModality {
    fn default() -> Self {
        Self {
            focus_visible: true,
        }
    }
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlRuntimeState {
    pub hovered: bool,
    pub active: bool,
    pub disabled: bool,
    pub focused: bool,
    pub focus_visible: bool,
    pub checked: bool,
    pub selected: bool,
    pub open: bool,
    pub valid: bool,
    pub invalid: bool,
    pub focus_within: bool,
    pub ancestor_checked: bool,
    pub ancestor_focus_within: bool,
    pub style_generation: u64,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct UiXmlTransitionState {
    pub property: TransitionProperty,
    pub from: Color,
    pub to: Color,
    pub elapsed: f32,
    pub duration: f32,
}

#[derive(Component, Debug, Clone)]
pub struct UiXmlStyleSource {
    pub base: UiStyle,
    pub hover: UiStyle,
    pub active: UiStyle,
    pub focus: UiStyle,
    pub checked: UiStyle,
    pub selected: UiStyle,
    pub open: UiStyle,
    pub valid: UiStyle,
    pub invalid: UiStyle,
    pub focus_within: UiStyle,
    pub focus_visible: UiStyle,
    pub ancestor_checked: UiStyle,
    pub ancestor_focus_within: UiStyle,
    pub disabled: UiStyle,
}

#[derive(Clone, Copy)]
pub(crate) struct RuntimeStyleInputs<'a> {
    pub(crate) base: &'a UiStyle,
    pub(crate) hover: &'a UiStyle,
    pub(crate) active: &'a UiStyle,
    pub(crate) focus: &'a UiStyle,
    pub(crate) checked: &'a UiStyle,
    pub(crate) selected: &'a UiStyle,
    pub(crate) open: &'a UiStyle,
    pub(crate) valid: &'a UiStyle,
    pub(crate) invalid: &'a UiStyle,
    pub(crate) focus_within: &'a UiStyle,
    pub(crate) focus_visible: &'a UiStyle,
    pub(crate) ancestor_checked: &'a UiStyle,
    pub(crate) ancestor_focus_within: &'a UiStyle,
    pub(crate) disabled: &'a UiStyle,
}

impl UiXmlStyleSource {
    pub(crate) fn from_runtime_styles(styles: RuntimeStyleInputs<'_>) -> Self {
        Self {
            base: styles.base.without_state_styles(),
            hover: state_overlay(styles.hover, styles.base.hover.as_deref()),
            active: state_overlay(styles.active, styles.base.active.as_deref()),
            focus: state_overlay(styles.focus, styles.base.focus.as_deref()),
            checked: state_overlay(styles.checked, styles.base.checked.as_deref()),
            selected: state_overlay(styles.selected, styles.base.selected.as_deref()),
            open: state_overlay(styles.open, styles.base.open.as_deref()),
            valid: state_overlay(styles.valid, styles.base.valid.as_deref()),
            invalid: state_overlay(styles.invalid, styles.base.invalid.as_deref()),
            focus_within: state_overlay(styles.focus_within, styles.base.focus_within.as_deref()),
            focus_visible: state_overlay(
                styles.focus_visible,
                styles.base.focus_visible.as_deref(),
            ),
            ancestor_checked: styles.ancestor_checked.without_state_styles(),
            ancestor_focus_within: styles.ancestor_focus_within.without_state_styles(),
            disabled: state_overlay(styles.disabled, styles.base.disabled.as_deref()),
        }
    }

    pub(crate) fn resolve(&self, runtime_state: UiXmlRuntimeState) -> UiStyle {
        let mut style = self.base.clone();
        if runtime_state.disabled {
            style.merge(&self.disabled);
            return style;
        }
        if runtime_state.checked {
            style.merge(&self.checked);
        }
        if runtime_state.selected {
            style.merge(&self.selected);
        }
        if runtime_state.open {
            style.merge(&self.open);
        }
        if runtime_state.valid {
            style.merge(&self.valid);
        }
        if runtime_state.invalid {
            style.merge(&self.invalid);
        }
        if runtime_state.ancestor_checked {
            style.merge(&self.ancestor_checked);
        }
        if runtime_state.focus_within {
            style.merge(&self.focus_within);
        }
        if runtime_state.ancestor_focus_within {
            style.merge(&self.ancestor_focus_within);
        }
        if runtime_state.focused {
            style.merge(&self.focus);
        }
        if runtime_state.focus_visible {
            style.merge(&self.focus_visible);
        }
        if runtime_state.active {
            style.merge(&self.active);
        } else if runtime_state.hovered {
            style.merge(&self.hover);
        }
        style
    }
}

fn state_overlay(state: &UiStyle, nested: Option<&UiStyle>) -> UiStyle {
    let mut overlay = state.without_state_styles();
    if let Some(nested) = nested {
        overlay.merge(&nested.without_state_styles());
    }
    overlay
}

#[derive(Resource, Debug, Clone, Default)]
pub struct UiXmlStyleRuntime {
    pub generation: u64,
}

#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlSelectorContext {
    pub parent: Option<Entity>,
    pub ancestors: Vec<UiXmlSelectorSnapshot>,
    pub tag: String,
    pub widget_type: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: HashMap<String, String>,
    pub state_generation: u64,
    pub style_generation: u64,
}

impl UiXmlSelectorContext {
    pub(crate) fn from_node(
        node: &ElementNode,
        parent: Option<Entity>,
        ancestors: &[&ElementNode],
    ) -> Self {
        Self {
            parent,
            ancestors: ancestors
                .iter()
                .map(|ancestor| (*ancestor).into())
                .collect(),
            tag: node.tag.clone(),
            widget_type: node.widget_type().to_string(),
            id: node.id.clone(),
            classes: node.classes.clone(),
            attributes: node.attributes.clone(),
            state_generation: 0,
            style_generation: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiXmlSelectorSnapshot {
    pub tag: String,
    pub widget_type: String,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: HashMap<String, String>,
}

impl From<&ElementNode> for UiXmlSelectorSnapshot {
    fn from(node: &ElementNode) -> Self {
        Self {
            tag: node.tag.clone(),
            widget_type: node.widget_type().to_string(),
            id: node.id.clone(),
            classes: node.classes.clone(),
            attributes: node.attributes.clone(),
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct UiXmlSelectorContextCache {
    pub generation: u64,
    pub entities: HashMap<Entity, UiXmlSelectorContext>,
}

impl From<&ElementNode> for UiXmlElement {
    fn from(node: &ElementNode) -> Self {
        Self {
            tag: node.tag.clone(),
            widget_type: node.widget_type().to_string(),
            id: node.id.clone(),
            classes: node.classes.clone(),
            attributes: node.attributes.clone(),
        }
    }
}

#[allow(dead_code)]
#[derive(Component, Debug, Clone)]
pub struct UiXmlStateStyles {
    base: UiStyle,
    hover: Option<UiStyle>,
    active: Option<UiStyle>,
    disabled: Option<UiStyle>,
    focus: Option<UiStyle>,
    checked: Option<UiStyle>,
    selected: Option<UiStyle>,
    open: Option<UiStyle>,
    valid: Option<UiStyle>,
    invalid: Option<UiStyle>,
    focus_within: Option<UiStyle>,
    focus_visible: Option<UiStyle>,
    ancestor_checked: Option<UiStyle>,
    ancestor_focus_within: Option<UiStyle>,
}

#[allow(dead_code)]
impl UiXmlStateStyles {
    #[cfg(test)]
    pub(crate) fn from_style(style: &UiStyle) -> Self {
        Self {
            base: style.without_state_styles(),
            hover: style.hover.as_deref().map(UiStyle::without_state_styles),
            active: style.active.as_deref().map(UiStyle::without_state_styles),
            disabled: style.disabled.as_deref().map(UiStyle::without_state_styles),
            focus: style.focus.as_deref().map(UiStyle::without_state_styles),
            checked: style.checked.as_deref().map(UiStyle::without_state_styles),
            selected: style.selected.as_deref().map(UiStyle::without_state_styles),
            open: style.open.as_deref().map(UiStyle::without_state_styles),
            valid: style.valid.as_deref().map(UiStyle::without_state_styles),
            invalid: style.invalid.as_deref().map(UiStyle::without_state_styles),
            focus_within: style
                .focus_within
                .as_deref()
                .map(UiStyle::without_state_styles),
            focus_visible: style
                .focus_visible
                .as_deref()
                .map(UiStyle::without_state_styles),
            ancestor_checked: None,
            ancestor_focus_within: None,
        }
    }

    pub(crate) fn from_runtime_styles(styles: RuntimeStyleInputs<'_>) -> Self {
        Self {
            base: styles.base.without_state_styles(),
            hover: Some(state_overlay(styles.hover, styles.base.hover.as_deref())),
            active: Some(state_overlay(styles.active, styles.base.active.as_deref())),
            focus: Some(state_overlay(styles.focus, styles.base.focus.as_deref())),
            checked: Some(state_overlay(
                styles.checked,
                styles.base.checked.as_deref(),
            )),
            selected: Some(state_overlay(
                styles.selected,
                styles.base.selected.as_deref(),
            )),
            open: Some(state_overlay(styles.open, styles.base.open.as_deref())),
            valid: Some(state_overlay(styles.valid, styles.base.valid.as_deref())),
            invalid: Some(state_overlay(
                styles.invalid,
                styles.base.invalid.as_deref(),
            )),
            focus_within: Some(state_overlay(
                styles.focus_within,
                styles.base.focus_within.as_deref(),
            )),
            focus_visible: Some(state_overlay(
                styles.focus_visible,
                styles.base.focus_visible.as_deref(),
            )),
            ancestor_checked: Some(styles.ancestor_checked.without_state_styles()),
            ancestor_focus_within: Some(styles.ancestor_focus_within.without_state_styles()),
            disabled: Some(state_overlay(
                styles.disabled,
                styles.base.disabled.as_deref(),
            )),
        }
    }

    pub(crate) fn resolve(&self, interaction: Interaction, disabled: bool) -> UiStyle {
        let mut style = self.base.clone();
        if disabled {
            if let Some(disabled) = &self.disabled {
                style.merge(disabled);
            }
            return style;
        }
        if let Some(checked) = &self.checked {
            style.merge(checked);
        }
        if let Some(selected) = &self.selected {
            style.merge(selected);
        }
        if let Some(open) = &self.open {
            style.merge(open);
        }
        if let Some(valid) = &self.valid {
            style.merge(valid);
        }
        if let Some(invalid) = &self.invalid {
            style.merge(invalid);
        }
        if let Some(ancestor_checked) = &self.ancestor_checked {
            style.merge(ancestor_checked);
        }
        if let Some(focus_within) = &self.focus_within {
            style.merge(focus_within);
        }
        if let Some(ancestor_focus_within) = &self.ancestor_focus_within {
            style.merge(ancestor_focus_within);
        }

        match interaction {
            Interaction::Pressed => {
                if let Some(active) = &self.active {
                    style.merge(active);
                }
            }
            Interaction::Hovered => {
                if let Some(hover) = &self.hover {
                    style.merge(hover);
                }
            }
            Interaction::None => {}
        }

        style
    }
}
pub struct UiXmlPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum UiXmlSystemSet {
    Prep,
    Input,
    TextForm,
    Style,
}

impl Plugin for UiXmlPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UiXmlControlChanged>()
            .add_event::<UiXmlFocusChanged>()
            .add_event::<UiXmlActivateRequested>()
            .add_event::<UiXmlBackRequested>()
            .add_event::<UiXmlSelectChanged>()
            .add_event::<UiXmlRangeChanged>()
            .add_event::<UiXmlScrollRequested>()
            .add_event::<UiXmlTextChanged>()
            .add_event::<UiXmlFormSubmitRequested>()
            .add_event::<UiXmlFormResetRequested>()
            .add_event::<UiXmlFormSubmitted>()
            .add_event::<UiXmlFormValidationFailed>()
            .add_event::<UiXmlValidationStateChanged>()
            .add_event::<UiXmlNavigationRequested>()
            .add_event::<UiXmlClipboardCopyRequested>()
            .add_event::<UiXmlClipboardCutRequested>()
            .add_event::<UiXmlClipboardPasteRequested>()
            .add_event::<UiXmlTextSelectAllRequested>()
            .add_event::<ReceivedCharacter>()
            .add_event::<KeyboardInput>()
            .add_event::<GamepadButtonInput>()
            .add_event::<GamepadAxisChangedEvent>()
            .add_event::<Ime>()
            .init_resource::<UiXmlClipboard>()
            .init_resource::<UiXmlThemeTokens>()
            .init_resource::<UiXmlStyleRuntime>()
            .init_resource::<UiXmlFocus>()
            .init_resource::<UiXmlInputModality>()
            .init_resource::<UiXmlSelectorContextCache>()
            .configure_sets(
                Update,
                (
                    UiXmlSystemSet::Prep,
                    UiXmlSystemSet::Input,
                    UiXmlSystemSet::TextForm,
                    UiXmlSystemSet::Style,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                (
                    register_selector_contexts,
                    mark_theme_runtime_generation,
                    mark_style_runtime_generation,
                    normalize_initial_radio_groups,
                )
                    .chain()
                    .in_set(UiXmlSystemSet::Prep),
            )
            .add_systems(
                Update,
                (
                    focus_pressed_focusables,
                    apply_control_interactions,
                    apply_focus_navigation,
                    apply_select_activation,
                    apply_range_input,
                    apply_scroll_requests,
                )
                    .chain()
                    .in_set(UiXmlSystemSet::Input),
            )
            .add_systems(
                Update,
                (
                    apply_text_selection_requests,
                    apply_clipboard_requests,
                    apply_text_input,
                    apply_ime_input,
                    apply_form_reset_requests,
                    apply_form_submit_requests,
                )
                    .chain()
                    .in_set(UiXmlSystemSet::TextForm),
            )
            .add_systems(
                Update,
                (
                    sync_text_display,
                    sync_scalar_fill_percent,
                    sync_runtime_state,
                    apply_interaction_styles,
                    update_transition_styles,
                    apply_effect_materials,
                )
                    .chain()
                    .in_set(UiXmlSystemSet::Style),
            );
    }
}

type InitialRadioQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlControlKind,
        &'static UiXmlDisabled,
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        &'static UiXmlDocumentOrder,
        &'static UiXmlChecked,
    ),
>;

type AddedControlQuery<'w, 's> =
    Query<'w, 's, Entity, (With<UiXmlControlKind>, Added<UiXmlChecked>)>;

type CheckedMutationQuery<'w, 's> = Query<'w, 's, &'static mut UiXmlChecked>;

fn normalize_initial_radio_groups(
    mut params: ParamSet<(
        AddedControlQuery<'_, '_>,
        InitialRadioQuery<'_, '_>,
        CheckedMutationQuery<'_, '_>,
    )>,
) {
    if params.p0().iter().next().is_none() {
        return;
    }

    let mut winners: HashMap<(Entity, String), (Entity, usize)> = HashMap::new();
    let mut checked_radios = Vec::new();

    for (entity, kind, disabled, scope, name, order, checked) in &params.p1() {
        if *kind != UiXmlControlKind::Radio || disabled.0 || !checked.0 {
            continue;
        }
        let Some(name) = normalized_name(name) else {
            continue;
        };
        checked_radios.push(entity);
        let key = (scope.0, name.to_string());
        match winners.get(&key) {
            Some((_, current_order)) if *current_order > order.0 => {}
            _ => {
                winners.insert(key, (entity, order.0));
            }
        }
    }

    let winner_entities = winners
        .values()
        .map(|(entity, _)| *entity)
        .collect::<HashSet<_>>();
    for entity in checked_radios {
        if winner_entities.contains(&entity) {
            continue;
        }
        if let Ok(mut checked) = params.p2().get_mut(entity) {
            checked.0 = false;
        }
    }
}

type ControlInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static Interaction,
        &'static UiXmlControlKind,
        &'static UiXmlDisabled,
        &'static UiXmlChecked,
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        Option<&'static UiXmlControlValue>,
    ),
    Changed<Interaction>,
>;

type ControlStateMutationQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlControlKind,
        &'static UiXmlDisabled,
        &'static mut UiXmlChecked,
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        Option<&'static UiXmlControlValue>,
    ),
>;

fn apply_control_interactions(
    mut events: EventWriter<UiXmlControlChanged>,
    mut params: ParamSet<(
        ControlInteractionQuery<'_, '_>,
        ControlStateMutationQuery<'_, '_>,
    )>,
) {
    let changes = params
        .p0()
        .iter()
        .filter_map(
            |(entity, interaction, kind, disabled, checked, scope, name, value)| {
                if *interaction != Interaction::Pressed || disabled.0 {
                    return None;
                }
                Some(ControlInteractionChange {
                    entity,
                    kind: *kind,
                    checked: checked.0,
                    scope: scope.0,
                    name: name.cloned(),
                    value: control_value(value),
                })
            },
        )
        .collect::<Vec<_>>();

    for change in changes {
        match change.kind {
            UiXmlControlKind::Checkbox => {
                if let Ok((_, _, _, mut checked, _, _, _)) = params.p1().get_mut(change.entity) {
                    let previous_checked = checked.0;
                    checked.0 = !checked.0;
                    events.send(UiXmlControlChanged {
                        entity: change.entity,
                        kind: change.kind,
                        scope: change.scope,
                        name: change.name.map(|name| name.0),
                        value: change.value,
                        checked: checked.0,
                        previous_checked,
                    });
                }
            }
            UiXmlControlKind::Radio => {
                if change.checked {
                    continue;
                }
                let group_name = change.name.as_ref().and_then(|name| {
                    let trimmed = name.0.trim();
                    (!trimmed.is_empty()).then_some(trimmed.to_string())
                });
                let mut peer_events = Vec::new();
                for (entity, kind, disabled, mut checked, scope, name, value) in &mut params.p1() {
                    if *kind != UiXmlControlKind::Radio {
                        continue;
                    }
                    if disabled.0 {
                        continue;
                    }
                    let same_entity = entity == change.entity;
                    let same_group = group_name.as_deref().is_some_and(|group_name| {
                        scope.0 == change.scope && normalized_name(name) == Some(group_name)
                    });
                    if !same_entity && !same_group {
                        continue;
                    }

                    let next_checked = same_entity;
                    if checked.0 == next_checked {
                        continue;
                    }
                    let previous_checked = checked.0;
                    checked.0 = next_checked;
                    peer_events.push(UiXmlControlChanged {
                        entity,
                        kind: *kind,
                        scope: scope.0,
                        name: name.cloned().map(|name| name.0),
                        value: control_value(value),
                        checked: checked.0,
                        previous_checked,
                    });
                }
                for event in peer_events {
                    events.send(event);
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
struct ControlInteractionChange {
    entity: Entity,
    kind: UiXmlControlKind,
    checked: bool,
    scope: Entity,
    name: Option<UiXmlControlName>,
    value: String,
}

fn normalized_name(name: Option<&UiXmlControlName>) -> Option<&str> {
    name.map(|name| name.0.trim())
        .filter(|name| !name.is_empty())
}

fn control_value(value: Option<&UiXmlControlValue>) -> String {
    value
        .map(|value| value.0.clone())
        .unwrap_or_else(|| "on".to_string())
}

fn apply_select_activation(
    mut activations: EventReader<UiXmlActivateRequested>,
    mut changed: EventWriter<UiXmlSelectChanged>,
    cache: Res<UiXmlSelectorContextCache>,
    mut selects: Query<(&mut UiXmlOpen, &mut UiXmlSelectValue), With<UiXmlSelect>>,
    mut options: Query<(Entity, &UiXmlOption, &mut UiXmlSelected)>,
) {
    for activation in activations.read() {
        if let Ok((mut open, _)) = selects.get_mut(activation.entity) {
            open.0 = !open.0;
            continue;
        }

        let Ok((option_entity, option, mut selected)) = options.get_mut(activation.entity) else {
            continue;
        };
        let Some(select_entity) = ancestor_with_select(option_entity, &cache, &selects) else {
            continue;
        };
        let next_value = option.value.clone();
        let previous_value = {
            let Ok((mut open, mut select_value)) = selects.get_mut(select_entity) else {
                continue;
            };
            let previous_value = select_value.0.clone();
            select_value.0 = next_value.clone();
            open.0 = false;
            previous_value
        };
        selected.0 = true;
        for (peer_entity, _, mut peer_selected) in &mut options {
            if peer_entity != option_entity
                && ancestor_with_select(peer_entity, &cache, &selects) == Some(select_entity)
            {
                peer_selected.0 = false;
            }
        }
        changed.send(UiXmlSelectChanged {
            select: select_entity,
            option: option_entity,
            previous_value,
            value: next_value,
        });
    }
}

fn ancestor_with_select(
    entity: Entity,
    cache: &UiXmlSelectorContextCache,
    selects: &Query<(&mut UiXmlOpen, &mut UiXmlSelectValue), With<UiXmlSelect>>,
) -> Option<Entity> {
    let mut cursor = entity;
    while let Some(context) = cache.entities.get(&cursor) {
        let parent = context.parent?;
        if selects.contains(parent) {
            return Some(parent);
        }
        cursor = parent;
    }
    None
}

fn apply_range_input(
    focus: Res<UiXmlFocus>,
    mut keyboard_inputs: EventReader<KeyboardInput>,
    mut gamepad_buttons: EventReader<GamepadButtonInput>,
    mut changed: EventWriter<UiXmlRangeChanged>,
    mut query: Query<(
        Entity,
        &'static UiXmlRange,
        &'static mut UiXmlRangeValue,
        Option<&'static mut UiXmlFillPercent>,
    )>,
) {
    let Some(focused) = focus.entity else {
        keyboard_inputs.clear();
        gamepad_buttons.clear();
        return;
    };
    let Ok((entity, range, mut value, fill)) = query.get_mut(focused) else {
        return;
    };
    let mut delta_steps = 0.0;
    for input in keyboard_inputs.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }
        match input.key_code {
            KeyCode::ArrowRight | KeyCode::ArrowUp => delta_steps += 1.0,
            KeyCode::ArrowLeft | KeyCode::ArrowDown => delta_steps -= 1.0,
            _ => {}
        }
    }
    for input in gamepad_buttons.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }
        match input.button.button_type {
            GamepadButtonType::DPadRight | GamepadButtonType::DPadUp => delta_steps += 1.0,
            GamepadButtonType::DPadLeft | GamepadButtonType::DPadDown => delta_steps -= 1.0,
            _ => {}
        }
    }
    if delta_steps == 0.0 {
        return;
    }
    let previous_value = value.0;
    value.0 = snap_scalar(
        value.0 + delta_steps * range.step,
        range.min,
        range.max,
        range.step,
    );
    if let Some(mut fill) = fill {
        fill.0 = scalar_percent(value.0, range.min, range.max);
    }
    if (previous_value - value.0).abs() > f32::EPSILON {
        changed.send(UiXmlRangeChanged {
            entity,
            previous_value,
            value: value.0,
        });
    }
}

fn apply_scroll_requests(
    mut requests: EventReader<UiXmlScrollRequested>,
    mut query: Query<(
        &'static UiXmlScrollContainer,
        &'static mut UiXmlScrollOffset,
    )>,
) {
    for request in requests.read() {
        let Ok((container, mut offset)) = query.get_mut(request.entity) else {
            continue;
        };
        offset.0 = (offset.0 + request.delta).clamp(container.min, container.max);
    }
}

fn snap_scalar(value: f32, min: f32, max: f32, step: f32) -> f32 {
    let steps = ((value - min) / step.max(f32::EPSILON)).round();
    (min + steps * step).clamp(min, max)
}

fn scalar_percent(value: f32, min: f32, max: f32) -> f32 {
    ((value - min) / (max - min).max(f32::EPSILON)).clamp(0.0, 1.0)
}

fn format_scalar(value: f32) -> String {
    if value.fract().abs() <= f32::EPSILON {
        format!("{}", value as i32)
    } else {
        value.to_string()
    }
}

type TextEditDirectQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlDisabled,
        &'static mut UiXmlTextValue,
        &'static mut UiXmlTextCursor,
        &'static mut UiXmlTextSelection,
        Option<&'static UiXmlControlScope>,
        Option<&'static UiXmlControlName>,
    ),
    With<UiXmlTextInput>,
>;

fn apply_text_selection_requests(
    mut requests: EventReader<UiXmlTextSelectAllRequested>,
    mut query: Query<(
        &UiXmlTextValue,
        &mut UiXmlTextCursor,
        &mut UiXmlTextSelection,
    )>,
) {
    for request in requests.read() {
        let Ok((value, mut cursor, mut selection)) = query.get_mut(request.entity) else {
            continue;
        };
        selection.anchor = 0;
        selection.focus = value.0.chars().count();
        cursor.position = selection.focus;
    }
}

fn apply_clipboard_requests(
    mut copy_requests: EventReader<UiXmlClipboardCopyRequested>,
    mut cut_requests: EventReader<UiXmlClipboardCutRequested>,
    mut paste_requests: EventReader<UiXmlClipboardPasteRequested>,
    mut clipboard: ResMut<UiXmlClipboard>,
    mut events: EventWriter<UiXmlTextChanged>,
    mut query: TextEditDirectQuery<'_, '_>,
) {
    for request in copy_requests.read() {
        if let Ok((_, value, _, selection, _, _)) = query.get_mut(request.entity) {
            clipboard.text = selected_text(&value.0, *selection);
        }
    }
    for request in cut_requests.read() {
        let Ok((disabled, mut value, mut cursor, mut selection, scope, name)) =
            query.get_mut(request.entity)
        else {
            continue;
        };
        if disabled.0 {
            continue;
        }
        clipboard.text = selected_text(&value.0, *selection);
        if clipboard.text.is_empty() {
            continue;
        }
        let previous_value = value.0.clone();
        delete_selection(&mut value.0, &mut cursor, &mut selection);
        send_text_changed(
            &mut events,
            request.entity,
            scope,
            name,
            previous_value,
            value.0.clone(),
        );
    }
    for request in paste_requests.read() {
        let Ok((disabled, mut value, mut cursor, mut selection, scope, name)) =
            query.get_mut(request.entity)
        else {
            continue;
        };
        if disabled.0 || clipboard.text.is_empty() {
            continue;
        }
        let previous_value = value.0.clone();
        replace_selection_or_insert(&mut value.0, &mut cursor, &mut selection, &clipboard.text);
        send_text_changed(
            &mut events,
            request.entity,
            scope,
            name,
            previous_value,
            value.0.clone(),
        );
    }
}

fn selected_text(value: &str, selection: UiXmlTextSelection) -> String {
    let (start, end) = selection_bounds(selection);
    if start == end {
        return String::new();
    }
    value
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

type FocusPointerInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Interaction, &'static UiXmlDisabled),
    (With<UiXmlFocusable>, Changed<Interaction>),
>;

fn focus_pressed_focusables(
    mut focus: ResMut<UiXmlFocus>,
    mut modality: ResMut<UiXmlInputModality>,
    mut changed: EventWriter<UiXmlFocusChanged>,
    query: FocusPointerInteractionQuery<'_, '_>,
) {
    for (entity, interaction, disabled) in &query {
        if *interaction == Interaction::Pressed && !disabled.0 {
            let previous = focus.entity;
            focus.entity = Some(entity);
            modality.focus_visible = false;
            if previous != Some(entity) {
                changed.send(UiXmlFocusChanged {
                    previous,
                    current: entity,
                });
            }
        }
    }
}

type FocusableQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlFocusable,
        &'static UiXmlElement,
        &'static UiXmlDisabled,
        Option<&'static Visibility>,
        Option<&'static Style>,
        Option<&'static UiXmlTextInput>,
        Option<&'static UiXmlRange>,
    ),
>;

#[derive(SystemParam)]
struct FocusNavigationInputs<'w, 's> {
    keyboard_inputs: EventReader<'w, 's, KeyboardInput>,
    gamepad_buttons: EventReader<'w, 's, GamepadButtonInput>,
    gamepad_axes: EventReader<'w, 's, GamepadAxisChangedEvent>,
}

#[derive(SystemParam)]
struct FocusNavigationOutputs<'w, 's> {
    changed: EventWriter<'w, UiXmlFocusChanged>,
    activate: EventWriter<'w, UiXmlActivateRequested>,
    back: EventWriter<'w, UiXmlBackRequested>,
    #[system_param(ignore)]
    marker: std::marker::PhantomData<&'s ()>,
}

fn apply_focus_navigation(
    mut focus: ResMut<UiXmlFocus>,
    mut modality: ResMut<UiXmlInputModality>,
    mut inputs: FocusNavigationInputs<'_, '_>,
    mut outputs: FocusNavigationOutputs<'_, '_>,
    focusables: FocusableQuery<'_, '_>,
) {
    for input in inputs.keyboard_inputs.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }
        match input.key_code {
            KeyCode::Tab => move_focus(
                UiXmlNavigationDirection::Next,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            KeyCode::ArrowDown => move_focus(
                UiXmlNavigationDirection::Down,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            KeyCode::ArrowUp => move_focus(
                UiXmlNavigationDirection::Up,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            KeyCode::ArrowRight
                if !focused_is_text_input(focus.entity, &focusables)
                    && !focused_is_range(focus.entity, &focusables) =>
            {
                move_focus(
                    UiXmlNavigationDirection::Right,
                    &mut focus,
                    &mut modality,
                    &mut outputs.changed,
                    &focusables,
                )
            }
            KeyCode::ArrowLeft
                if !focused_is_text_input(focus.entity, &focusables)
                    && !focused_is_range(focus.entity, &focusables) =>
            {
                move_focus(
                    UiXmlNavigationDirection::Left,
                    &mut focus,
                    &mut modality,
                    &mut outputs.changed,
                    &focusables,
                )
            }
            KeyCode::Enter => {
                if let Some(entity) = focus.entity {
                    modality.focus_visible = true;
                    outputs.activate.send(UiXmlActivateRequested { entity });
                }
            }
            KeyCode::Escape => {
                modality.focus_visible = true;
                outputs.back.send(UiXmlBackRequested {
                    focused: focus.entity,
                });
            }
            _ => {}
        }
    }

    for input in inputs.gamepad_buttons.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }
        match input.button.button_type {
            GamepadButtonType::DPadDown => move_focus(
                UiXmlNavigationDirection::Down,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            GamepadButtonType::DPadUp => move_focus(
                UiXmlNavigationDirection::Up,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            GamepadButtonType::DPadRight => move_focus(
                UiXmlNavigationDirection::Right,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            GamepadButtonType::DPadLeft => move_focus(
                UiXmlNavigationDirection::Left,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            ),
            GamepadButtonType::South => {
                if let Some(entity) = focus.entity {
                    modality.focus_visible = true;
                    outputs.activate.send(UiXmlActivateRequested { entity });
                }
            }
            GamepadButtonType::East => {
                modality.focus_visible = true;
                outputs.back.send(UiXmlBackRequested {
                    focused: focus.entity,
                });
            }
            _ => {}
        }
    }

    for input in inputs.gamepad_axes.read() {
        let direction = match (input.axis_type, input.value) {
            (GamepadAxisType::LeftStickX, value) if value > 0.5 => {
                Some(UiXmlNavigationDirection::Right)
            }
            (GamepadAxisType::LeftStickX, value) if value < -0.5 => {
                Some(UiXmlNavigationDirection::Left)
            }
            (GamepadAxisType::LeftStickY, value) if value > 0.5 => {
                Some(UiXmlNavigationDirection::Up)
            }
            (GamepadAxisType::LeftStickY, value) if value < -0.5 => {
                Some(UiXmlNavigationDirection::Down)
            }
            _ => None,
        };
        if let Some(direction) = direction {
            move_focus(
                direction,
                &mut focus,
                &mut modality,
                &mut outputs.changed,
                &focusables,
            );
        }
    }
}

fn move_focus(
    direction: UiXmlNavigationDirection,
    focus: &mut UiXmlFocus,
    modality: &mut UiXmlInputModality,
    changed: &mut EventWriter<UiXmlFocusChanged>,
    focusables: &FocusableQuery<'_, '_>,
) {
    let candidates = focusable_candidates(focusables);
    if candidates.is_empty() {
        return;
    }
    let previous = focus.entity;
    let next = explicit_focus_target(previous, direction, &candidates)
        .or_else(|| fallback_focus_target(previous, direction, &candidates))
        .unwrap_or(candidates[0].entity);
    focus.entity = Some(next);
    modality.focus_visible = true;
    if previous != Some(next) {
        changed.send(UiXmlFocusChanged {
            previous,
            current: next,
        });
    }
}

#[derive(Clone)]
struct FocusCandidate {
    entity: Entity,
    id: Option<String>,
    order_key: (i32, usize),
    focusable: UiXmlFocusable,
}

fn focusable_candidates(focusables: &FocusableQuery<'_, '_>) -> Vec<FocusCandidate> {
    let mut candidates = focusables
        .iter()
        .filter(|(_, _, _, disabled, visibility, style, _, _)| {
            !disabled.0
                && !matches!(visibility, Some(Visibility::Hidden))
                && !matches!(style.map(|style| style.display), Some(Display::None))
        })
        .map(
            |(entity, focusable, element, _, _, _, _, _)| FocusCandidate {
                entity,
                id: element.id.clone(),
                order_key: (
                    focusable.tab_index.unwrap_or(focusable.order as i32),
                    focusable.order,
                ),
                focusable: focusable.clone(),
            },
        )
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| candidate.order_key);
    candidates
}

fn explicit_focus_target(
    current: Option<Entity>,
    direction: UiXmlNavigationDirection,
    candidates: &[FocusCandidate],
) -> Option<Entity> {
    let current = candidates
        .iter()
        .find(|candidate| Some(candidate.entity) == current)?;
    let target_id = match direction {
        UiXmlNavigationDirection::Up => current.focusable.up.as_deref(),
        UiXmlNavigationDirection::Down => current.focusable.down.as_deref(),
        UiXmlNavigationDirection::Left => current.focusable.left.as_deref(),
        UiXmlNavigationDirection::Right => current.focusable.right.as_deref(),
        UiXmlNavigationDirection::Next | UiXmlNavigationDirection::Previous => None,
    }?;
    candidates
        .iter()
        .find(|candidate| candidate.id.as_deref() == Some(target_id))
        .map(|candidate| candidate.entity)
}

fn fallback_focus_target(
    current: Option<Entity>,
    direction: UiXmlNavigationDirection,
    candidates: &[FocusCandidate],
) -> Option<Entity> {
    let current_index = current
        .and_then(|entity| {
            candidates
                .iter()
                .position(|candidate| candidate.entity == entity)
        })
        .unwrap_or(usize::MAX);
    if current_index == usize::MAX {
        return Some(candidates[0].entity);
    }
    match direction {
        UiXmlNavigationDirection::Previous
        | UiXmlNavigationDirection::Up
        | UiXmlNavigationDirection::Left => {
            Some(candidates[(current_index + candidates.len() - 1) % candidates.len()].entity)
        }
        UiXmlNavigationDirection::Next
        | UiXmlNavigationDirection::Down
        | UiXmlNavigationDirection::Right => {
            Some(candidates[(current_index + 1) % candidates.len()].entity)
        }
    }
}

fn focused_is_text_input(current: Option<Entity>, focusables: &FocusableQuery<'_, '_>) -> bool {
    current.is_some_and(|entity| {
        focusables
            .get(entity)
            .is_ok_and(|(_, _, _, _, _, _, text_input, _)| text_input.is_some())
    })
}

fn focused_is_range(current: Option<Entity>, focusables: &FocusableQuery<'_, '_>) -> bool {
    current.is_some_and(|entity| {
        focusables
            .get(entity)
            .is_ok_and(|(_, _, _, _, _, _, _, range)| range.is_some())
    })
}

type TextInputMutationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlDisabled,
        &'static mut UiXmlTextValue,
        &'static mut UiXmlTextCursor,
        &'static mut UiXmlTextSelection,
        Option<&'static UiXmlTextArea>,
        Option<&'static UiXmlControlScope>,
        Option<&'static UiXmlControlName>,
    ),
    With<UiXmlTextInput>,
>;

fn apply_text_input(
    focus: Res<UiXmlFocus>,
    mut modality: ResMut<UiXmlInputModality>,
    mut received_characters: EventReader<ReceivedCharacter>,
    mut keyboard_inputs: EventReader<KeyboardInput>,
    mut events: EventWriter<UiXmlTextChanged>,
    mut query: TextInputMutationQuery<'_, '_>,
) {
    let Some(entity) = focus.entity else {
        received_characters.clear();
        keyboard_inputs.clear();
        return;
    };

    let Ok((disabled, mut value, mut cursor, mut selection, text_area, scope, name)) =
        query.get_mut(entity)
    else {
        received_characters.clear();
        keyboard_inputs.clear();
        return;
    };

    if disabled.0 {
        received_characters.clear();
        keyboard_inputs.clear();
        return;
    }

    for received in received_characters.read() {
        if received.char.chars().any(char::is_control) {
            continue;
        }
        modality.focus_visible = true;
        let previous_value = value.0.clone();
        replace_selection_or_insert(
            &mut value.0,
            &mut cursor,
            &mut selection,
            received.char.as_str(),
        );
        send_text_changed(
            &mut events,
            entity,
            scope,
            name,
            previous_value,
            value.0.clone(),
        );
    }

    for input in keyboard_inputs.read() {
        if input.state != ButtonState::Pressed {
            continue;
        }
        modality.focus_visible = true;
        match input.key_code {
            KeyCode::Backspace => {
                let previous_value = value.0.clone();
                if delete_selection(&mut value.0, &mut cursor, &mut selection) {
                    send_text_changed(
                        &mut events,
                        entity,
                        scope,
                        name,
                        previous_value,
                        value.0.clone(),
                    );
                    continue;
                }
                if cursor.position == 0 || value.0.is_empty() {
                    continue;
                }
                let start = char_to_byte_index(&value.0, cursor.position - 1);
                let end = char_to_byte_index(&value.0, cursor.position);
                value.0.replace_range(start..end, "");
                cursor.position -= 1;
                selection.anchor = cursor.position;
                selection.focus = cursor.position;
                send_text_changed(
                    &mut events,
                    entity,
                    scope,
                    name,
                    previous_value,
                    value.0.clone(),
                );
            }
            KeyCode::Delete => {
                let previous_value = value.0.clone();
                if delete_selection(&mut value.0, &mut cursor, &mut selection) {
                    send_text_changed(
                        &mut events,
                        entity,
                        scope,
                        name,
                        previous_value,
                        value.0.clone(),
                    );
                    continue;
                }
                if cursor.position >= value.0.chars().count() {
                    continue;
                }
                let start = char_to_byte_index(&value.0, cursor.position);
                let end = char_to_byte_index(&value.0, cursor.position + 1);
                value.0.replace_range(start..end, "");
                selection.anchor = cursor.position;
                selection.focus = cursor.position;
                send_text_changed(
                    &mut events,
                    entity,
                    scope,
                    name,
                    previous_value,
                    value.0.clone(),
                );
            }
            KeyCode::ArrowLeft => {
                cursor.position = cursor.position.saturating_sub(1);
                selection.anchor = cursor.position;
                selection.focus = cursor.position;
            }
            KeyCode::ArrowRight => {
                cursor.position = (cursor.position + 1).min(value.0.chars().count());
                selection.anchor = cursor.position;
                selection.focus = cursor.position;
            }
            KeyCode::Home => {
                cursor.position = 0;
                selection.anchor = 0;
                selection.focus = 0;
            }
            KeyCode::End => {
                cursor.position = value.0.chars().count();
                selection.anchor = cursor.position;
                selection.focus = cursor.position;
            }
            KeyCode::Enter if text_area.is_some() => {
                let previous_value = value.0.clone();
                replace_selection_or_insert(&mut value.0, &mut cursor, &mut selection, "\n");
                send_text_changed(
                    &mut events,
                    entity,
                    scope,
                    name,
                    previous_value,
                    value.0.clone(),
                );
            }
            _ => {}
        }
    }
}

fn replace_selection_or_insert(
    value: &mut String,
    cursor: &mut UiXmlTextCursor,
    selection: &mut UiXmlTextSelection,
    text: &str,
) {
    let (start, end) = selection_bounds(*selection);
    if start != end {
        let byte_start = char_to_byte_index(value, start);
        let byte_end = char_to_byte_index(value, end);
        value.replace_range(byte_start..byte_end, text);
        cursor.position = start + text.chars().count();
    } else {
        let byte_index = char_to_byte_index(value, cursor.position);
        value.insert_str(byte_index, text);
        cursor.position += text.chars().count();
    }
    selection.anchor = cursor.position;
    selection.focus = cursor.position;
}

fn delete_selection(
    value: &mut String,
    cursor: &mut UiXmlTextCursor,
    selection: &mut UiXmlTextSelection,
) -> bool {
    let (start, end) = selection_bounds(*selection);
    if start == end {
        return false;
    }
    let byte_start = char_to_byte_index(value, start);
    let byte_end = char_to_byte_index(value, end);
    value.replace_range(byte_start..byte_end, "");
    cursor.position = start;
    selection.anchor = start;
    selection.focus = start;
    true
}

fn selection_bounds(selection: UiXmlTextSelection) -> (usize, usize) {
    (
        selection.anchor.min(selection.focus),
        selection.anchor.max(selection.focus),
    )
}

type ImeTextQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlDisabled,
        &'static mut UiXmlTextValue,
        &'static mut UiXmlTextCursor,
        &'static mut UiXmlTextSelection,
        &'static mut UiXmlImePreedit,
        Option<&'static UiXmlControlScope>,
        Option<&'static UiXmlControlName>,
    ),
    With<UiXmlTextInput>,
>;

fn apply_ime_input(
    focus: Res<UiXmlFocus>,
    mut modality: ResMut<UiXmlInputModality>,
    mut ime_events: EventReader<Ime>,
    mut text_events: EventWriter<UiXmlTextChanged>,
    mut query: ImeTextQuery<'_, '_>,
) {
    let Some(entity) = focus.entity else {
        ime_events.clear();
        return;
    };
    let Ok((disabled, mut value, mut cursor, mut selection, mut preedit, scope, name)) =
        query.get_mut(entity)
    else {
        ime_events.clear();
        return;
    };
    if disabled.0 {
        ime_events.clear();
        return;
    }

    for event in ime_events.read() {
        modality.focus_visible = true;
        match event {
            Ime::Preedit { value, cursor, .. } => {
                preedit.value = value.clone();
                preedit.cursor = *cursor;
            }
            Ime::Commit { value: text, .. } => {
                let previous_value = value.0.clone();
                replace_selection_or_insert(&mut value.0, &mut cursor, &mut selection, text);
                preedit.value.clear();
                preedit.cursor = None;
                send_text_changed(
                    &mut text_events,
                    entity,
                    scope,
                    name,
                    previous_value,
                    value.0.clone(),
                );
            }
            Ime::Enabled { .. } => {}
            Ime::Disabled { .. } => {
                preedit.value.clear();
                preedit.cursor = None;
            }
        }
    }
}

fn char_to_byte_index(value: &str, char_index: usize) -> usize {
    value
        .char_indices()
        .nth(char_index)
        .map(|(index, _)| index)
        .unwrap_or(value.len())
}

fn send_text_changed(
    events: &mut EventWriter<UiXmlTextChanged>,
    entity: Entity,
    scope: Option<&UiXmlControlScope>,
    name: Option<&UiXmlControlName>,
    previous_value: String,
    value: String,
) {
    events.send(UiXmlTextChanged {
        entity,
        scope: scope.map(|scope| scope.0).unwrap_or(entity),
        name: name.cloned().map(|name| name.0),
        previous_value,
        value,
    });
}

type FormTextQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        &'static UiXmlTextValue,
        Option<&'static UiXmlRequired>,
        Option<&'static mut UiXmlValidationState>,
    ),
    With<UiXmlTextInput>,
>;

type FormControlQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        Option<&'static UiXmlControlValue>,
        &'static UiXmlControlKind,
        &'static UiXmlChecked,
    ),
>;

type FormSelectQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        &'static UiXmlSelectValue,
    ),
    With<UiXmlSelect>,
>;

type FormRangeQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlControlScope,
        Option<&'static UiXmlControlName>,
        &'static UiXmlRangeValue,
    ),
    With<UiXmlRange>,
>;

type FormTextResetQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlControlScope,
        &'static UiXmlInitialTextValue,
        &'static mut UiXmlTextValue,
        Option<&'static mut UiXmlTextCursor>,
        Option<&'static mut UiXmlTextSelection>,
    ),
    With<UiXmlTextInput>,
>;

type FormControlResetQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlControlScope,
        &'static UiXmlInitialChecked,
        &'static mut UiXmlChecked,
    ),
    With<UiXmlControlKind>,
>;

fn apply_form_reset_requests(
    mut requests: EventReader<UiXmlFormResetRequested>,
    mut text_query: FormTextResetQuery<'_, '_>,
    mut control_query: FormControlResetQuery<'_, '_>,
) {
    for request in requests.read() {
        for (scope, initial, mut value, cursor, selection) in &mut text_query {
            if scope.0 != request.form {
                continue;
            }
            value.0 = initial.0.clone();
            if let Some(mut cursor) = cursor {
                cursor.position = value.0.chars().count();
            }
            if let Some(mut selection) = selection {
                selection.anchor = value.0.chars().count();
                selection.focus = selection.anchor;
            }
        }
        for (scope, initial, mut checked) in &mut control_query {
            if scope.0 == request.form {
                checked.0 = initial.0;
            }
        }
    }
}

#[derive(SystemParam)]
struct FormSubmitQueries<'w, 's> {
    text: FormTextQuery<'w, 's>,
    control: FormControlQuery<'w, 's>,
    select: FormSelectQuery<'w, 's>,
    range: FormRangeQuery<'w, 's>,
}

#[derive(SystemParam)]
struct FormSubmitOutputs<'w> {
    submitted: EventWriter<'w, UiXmlFormSubmitted>,
    validation_failed: EventWriter<'w, UiXmlFormValidationFailed>,
    validation_changed: EventWriter<'w, UiXmlValidationStateChanged>,
    navigation: EventWriter<'w, UiXmlNavigationRequested>,
}

fn apply_form_submit_requests(
    mut requests: EventReader<UiXmlFormSubmitRequested>,
    mut queries: FormSubmitQueries<'_, '_>,
    mut outputs: FormSubmitOutputs<'_>,
) {
    for request in requests.read() {
        let mut values = Vec::new();
        let mut valid = true;

        for (entity, scope, name, value, required, validation_state) in &mut queries.text {
            if scope.0 != request.form {
                continue;
            }
            let name_value = name
                .map(|name| name.0.trim())
                .filter(|name| !name.is_empty());
            if required.is_some_and(|required| required.0) && value.0.trim().is_empty() {
                valid = false;
                if let Some(mut validation_state) = validation_state {
                    validation_state.valid = false;
                    validation_state.reason = Some("required".to_string());
                }
                outputs
                    .validation_changed
                    .send(UiXmlValidationStateChanged {
                        entity,
                        valid: false,
                        reason: Some("required".to_string()),
                    });
                outputs.validation_failed.send(UiXmlFormValidationFailed {
                    form: request.form,
                    entity,
                    name: name_value.map(str::to_string),
                    reason: "required".to_string(),
                });
            } else if let Some(mut validation_state) = validation_state {
                validation_state.valid = true;
                validation_state.reason = None;
                outputs
                    .validation_changed
                    .send(UiXmlValidationStateChanged {
                        entity,
                        valid: true,
                        reason: None,
                    });
            }
            if let Some(name) = name_value {
                values.push(UiXmlFormValue {
                    name: name.to_string(),
                    value: value.0.clone(),
                });
            }
        }

        for (scope, name, value, kind, checked) in &queries.control {
            if scope.0 != request.form || !checked.0 {
                continue;
            }
            let Some(name) = normalized_name(name) else {
                continue;
            };
            let value = match kind {
                UiXmlControlKind::Checkbox | UiXmlControlKind::Radio => control_value(value),
            };
            values.push(UiXmlFormValue {
                name: name.to_string(),
                value,
            });
        }

        for (scope, name, value) in &queries.select {
            if scope.0 != request.form {
                continue;
            }
            let Some(name) = normalized_name(name) else {
                continue;
            };
            values.push(UiXmlFormValue {
                name: name.to_string(),
                value: value.0.clone(),
            });
        }

        for (scope, name, value) in &queries.range {
            if scope.0 != request.form {
                continue;
            }
            let Some(name) = normalized_name(name) else {
                continue;
            };
            values.push(UiXmlFormValue {
                name: name.to_string(),
                value: format_scalar(value.0),
            });
        }

        if valid {
            outputs.navigation.send(UiXmlNavigationRequested {
                form: request.form,
                action: None,
                method: None,
                values: values.clone(),
            });
            outputs.submitted.send(UiXmlFormSubmitted {
                form: request.form,
                values,
            });
        }
    }
}

type TextDisplaySyncQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlTextValue,
        &'static UiXmlTextDisplay,
        Option<&'static UiXmlTextPlaceholder>,
        Option<&'static mut UiXmlTextCursor>,
    ),
    Changed<UiXmlTextValue>,
>;

fn sync_text_display(
    mut value_query: TextDisplaySyncQuery<'_, '_>,
    mut text_query: Query<&mut Text>,
) {
    for (value, display, placeholder, cursor) in &mut value_query {
        if let Some(mut cursor) = cursor {
            cursor.position = cursor.position.min(value.0.chars().count());
        }
        let Ok(mut text) = text_query.get_mut(display.0) else {
            continue;
        };
        if let Some(section) = text.sections.first_mut() {
            apply_text_presentation(section, value, placeholder);
        }
    }
}

type ScalarFillQuery<'w, 's, T> =
    Query<'w, 's, (&'static T, &'static mut UiXmlFillPercent), Changed<T>>;

fn sync_scalar_fill_percent(
    mut params: ParamSet<(
        ScalarFillQuery<'_, '_, UiXmlProgress>,
        ScalarFillQuery<'_, '_, UiXmlMeter>,
    )>,
) {
    for (progress, mut fill) in &mut params.p0() {
        fill.0 = scalar_percent(progress.value.clamp(0.0, progress.max), 0.0, progress.max);
    }
    for (meter, mut fill) in &mut params.p1() {
        fill.0 = scalar_percent(
            meter.value.clamp(meter.min, meter.max),
            meter.min,
            meter.max,
        );
    }
}

pub(crate) fn apply_text_presentation(
    section: &mut TextSection,
    value: &UiXmlTextValue,
    placeholder: Option<&UiXmlTextPlaceholder>,
) {
    let show_placeholder = value.0.is_empty();
    if show_placeholder {
        if let Some(placeholder) = placeholder {
            section.value = placeholder.text.clone();
            section.style.color = placeholder
                .placeholder_color
                .unwrap_or(placeholder.value_color);
            section.style.font_size = placeholder
                .placeholder_font_size
                .unwrap_or(placeholder.value_font_size);
            return;
        }
    }

    section.value = value.0.clone();
    if let Some(placeholder) = placeholder {
        section.style.color = placeholder.value_color;
        section.style.font_size = placeholder.value_font_size;
    }
}

type SelectorContextRegistrationQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static UiXmlSelectorContext),
    Or<(Added<UiXmlSelectorContext>, Changed<UiXmlSelectorContext>)>,
>;

fn register_selector_contexts(
    mut cache: ResMut<UiXmlSelectorContextCache>,
    query: SelectorContextRegistrationQuery<'_, '_>,
) {
    for (entity, context) in &query {
        cache.entities.insert(entity, context.clone());
        cache.generation += 1;
    }
}

fn mark_style_runtime_generation(
    runtime: Res<UiXmlStyleRuntime>,
    mut query: Query<&mut UiXmlRuntimeState>,
) {
    if !runtime.is_changed() {
        return;
    }

    for mut state in &mut query {
        state.style_generation = runtime.generation;
    }
}

fn mark_theme_runtime_generation(
    theme: Res<UiXmlThemeTokens>,
    mut runtime: ResMut<UiXmlStyleRuntime>,
) {
    if theme.is_changed() && !theme.tokens.is_empty() {
        runtime.generation += 1;
    }
}

type RuntimeStateQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static Interaction>,
        &'static UiXmlDisabled,
        Option<&'static UiXmlChecked>,
        Option<&'static UiXmlSelected>,
        Option<&'static UiXmlOpen>,
        Option<&'static UiXmlValidationState>,
        &'static mut UiXmlRuntimeState,
    ),
>;

fn sync_runtime_state(
    focus: Res<UiXmlFocus>,
    modality: Res<UiXmlInputModality>,
    cache: Res<UiXmlSelectorContextCache>,
    checked_lookup: Query<&UiXmlChecked>,
    mut query: RuntimeStateQuery<'_, '_>,
) {
    for (entity, interaction, disabled, checked, selected, open, validation, mut state) in
        &mut query
    {
        let next_disabled = disabled.0;
        let next_focused = focus.entity == Some(entity) && !next_disabled;
        let next_focus_visible = next_focused && modality.focus_visible;
        let next_checked = checked.is_some_and(|checked| checked.0) && !next_disabled;
        let next_selected = selected.is_some_and(|selected| selected.0) && !next_disabled;
        let next_open = open.is_some_and(|open| open.0) && !next_disabled;
        let next_valid = validation.is_some_and(|validation| validation.valid) && !next_disabled;
        let next_invalid = validation.is_some_and(|validation| !validation.valid) && !next_disabled;
        let next_focus_within = focus
            .entity
            .is_some_and(|focused| entity_contains_focus(entity, focused, &cache))
            && !next_disabled;
        let next_ancestor_focus_within = focus
            .entity
            .is_some_and(|focused| any_ancestor_contains_focus(entity, focused, &cache))
            && !next_disabled;
        let next_ancestor_checked =
            ancestor_checked(entity, &cache, &checked_lookup) && !next_disabled;
        let (next_active, next_hovered) = if next_disabled {
            (false, false)
        } else {
            match interaction.copied().unwrap_or(Interaction::None) {
                Interaction::Pressed => (true, false),
                Interaction::Hovered => (false, true),
                Interaction::None => (false, false),
            }
        };

        if state.disabled == next_disabled
            && state.focused == next_focused
            && state.focus_visible == next_focus_visible
            && state.checked == next_checked
            && state.selected == next_selected
            && state.open == next_open
            && state.valid == next_valid
            && state.invalid == next_invalid
            && state.focus_within == next_focus_within
            && state.ancestor_checked == next_ancestor_checked
            && state.ancestor_focus_within == next_ancestor_focus_within
            && state.active == next_active
            && state.hovered == next_hovered
        {
            continue;
        }

        state.disabled = next_disabled;
        state.focused = next_focused;
        state.focus_visible = next_focus_visible;
        state.checked = next_checked;
        state.selected = next_selected;
        state.open = next_open;
        state.valid = next_valid;
        state.invalid = next_invalid;
        state.focus_within = next_focus_within;
        state.ancestor_checked = next_ancestor_checked;
        state.ancestor_focus_within = next_ancestor_focus_within;
        state.active = next_active;
        state.hovered = next_hovered;
        state.set_changed();
        if disabled.0 {
            continue;
        }
    }
}

fn entity_contains_focus(
    entity: Entity,
    focused: Entity,
    cache: &UiXmlSelectorContextCache,
) -> bool {
    if entity == focused {
        return true;
    }

    let mut cursor = focused;
    while let Some(context) = cache.entities.get(&cursor) {
        let Some(parent) = context.parent else {
            return false;
        };
        if parent == entity {
            return true;
        }
        cursor = parent;
    }
    false
}

fn any_ancestor_contains_focus(
    entity: Entity,
    focused: Entity,
    cache: &UiXmlSelectorContextCache,
) -> bool {
    let mut cursor = entity;
    while let Some(context) = cache.entities.get(&cursor) {
        let Some(parent) = context.parent else {
            return false;
        };
        if entity_contains_focus(parent, focused, cache) {
            return true;
        }
        cursor = parent;
    }
    false
}

fn ancestor_checked(
    entity: Entity,
    cache: &UiXmlSelectorContextCache,
    checked_lookup: &Query<&UiXmlChecked>,
) -> bool {
    let mut cursor = entity;
    while let Some(context) = cache.entities.get(&cursor) {
        let Some(parent) = context.parent else {
            return false;
        };
        if checked_lookup.get(parent).is_ok_and(|checked| checked.0) {
            return true;
        }
        cursor = parent;
    }
    false
}

type InteractionStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlRuntimeState,
        &'static UiXmlStyleSource,
        Option<&'static UiXmlTransitionState>,
        &'static mut Style,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        Option<&'static mut Outline>,
        Option<&'static mut crate::render_effects::UiXmlUnsupportedEffects>,
        Option<&'static mut crate::render_effects::UiXmlBorderColors>,
        Option<&'static mut crate::render_effects::UiXmlRenderMaterialSpec>,
    ),
    Or<(Changed<UiXmlRuntimeState>, Changed<UiXmlStyleSource>)>,
>;

fn apply_interaction_styles(
    mut commands: Commands<'_, '_>,
    mut query: InteractionStyleQuery<'_, '_>,
) {
    for (
        entity,
        runtime_state,
        style_source,
        maybe_transition,
        mut bevy_style,
        mut background,
        mut border,
        maybe_outline,
        maybe_effects,
        maybe_border_colors,
        maybe_material_spec,
    ) in &mut query
    {
        let resolved = style_source.resolve(*runtime_state);
        *bevy_style = to_bevy_style(&resolved);
        let next_background = style_color(
            resolved.background.as_deref(),
            Color::NONE,
            resolved.opacity,
        );
        if let Some(transition) = resolved.transition.as_ref().filter(|transition| {
            matches!(
                transition.property,
                TransitionProperty::Background | TransitionProperty::Opacity
            ) && transition.duration > 0.0
        }) {
            if maybe_transition.is_none()
                && background.0.as_rgba_u8() != next_background.as_rgba_u8()
            {
                commands.entity(entity).insert(UiXmlTransitionState {
                    property: transition.property,
                    from: background.0,
                    to: next_background,
                    elapsed: 0.0,
                    duration: transition.duration,
                });
            }
        } else {
            background.0 = next_background;
            commands.entity(entity).remove::<UiXmlTransitionState>();
        }
        border.0 = style_color(
            resolved.border_color.as_deref(),
            Color::NONE,
            resolved.opacity,
        );
        let outline = outline_from_style(&resolved);
        match (maybe_outline, outline) {
            (Some(mut current), Some(next)) => {
                *current = next;
            }
            (None, Some(next)) => {
                commands.entity(entity).insert(next);
            }
            (Some(mut current), None) => {
                current.color = Color::NONE;
            }
            (None, None) => {}
        }
        match (maybe_effects, unsupported_effects_from_style(&resolved)) {
            (Some(mut current), Some(next)) => *current = next,
            (None, Some(next)) => {
                commands.entity(entity).insert(next);
            }
            (Some(mut current), None) => current.effects.clear(),
            (None, None) => {}
        }
        match (maybe_border_colors, border_colors_from_style(&resolved)) {
            (Some(mut current), Some(next)) => *current = next,
            (None, Some(next)) => {
                commands.entity(entity).insert(next);
            }
            (Some(mut current), None) => {
                current.top = None;
                current.right = None;
                current.bottom = None;
                current.left = None;
            }
            (None, None) => {}
        }
        match (
            maybe_material_spec,
            render_material_spec_from_style(&resolved),
        ) {
            (Some(mut current), Some(next)) => *current = next,
            (None, Some(next)) => {
                commands.entity(entity).insert(next);
            }
            (Some(mut current), None) => {
                current.background = None;
                current.border_color = None;
                current.border_radius = None;
                current.box_shadow = None;
                current.filter = None;
                current.backdrop_filter = None;
                current.gradient = None;
                current.gradient_end = None;
            }
            (None, None) => {}
        }
    }
}

fn update_transition_styles(
    time: Option<Res<Time>>,
    mut commands: Commands<'_, '_>,
    mut query: Query<(
        Entity,
        &'static mut UiXmlTransitionState,
        &'static mut BackgroundColor,
    )>,
) {
    let delta = time
        .as_deref()
        .map(Time::delta_seconds)
        .unwrap_or(1.0 / 60.0);
    for (entity, mut transition, mut background) in &mut query {
        transition.elapsed = (transition.elapsed + delta).min(transition.duration);
        let t = if transition.duration <= f32::EPSILON {
            1.0
        } else {
            transition.elapsed / transition.duration
        };
        background.0 = lerp_color(transition.from, transition.to, t);
        if transition.elapsed >= transition.duration {
            background.0 = transition.to;
            commands.entity(entity).remove::<UiXmlTransitionState>();
        }
    }
}

fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    let [fr, fg, fb, fa] = from.as_rgba_f32();
    let [tr, tg, tb, ta] = to.as_rgba_f32();
    let t = t.clamp(0.0, 1.0);
    Color::rgba(
        fr + (tr - fr) * t,
        fg + (tg - fg) * t,
        fb + (tb - fb) * t,
        fa + (ta - fa) * t,
    )
}

type EffectMaterialQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlRuntimeState,
        &'static UiXmlStyleSource,
        Option<&'static mut UiXmlRenderMaterialSpec>,
        Option<&'static Handle<UiXmlEffectMaterial>>,
        &'static mut Style,
        Option<&'static mut BorderColor>,
        Option<&'static mut BackgroundColor>,
    ),
    (
        With<UiXmlRenderMaterialSpec>,
        Or<(Changed<UiXmlRuntimeState>, Changed<UiXmlStyleSource>)>,
    ),
>;

fn apply_effect_materials(
    mut commands: Commands<'_, '_>,
    materials: Option<ResMut<Assets<UiXmlEffectMaterial>>>,
    mut query: EffectMaterialQuery<'_, '_>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (
        entity,
        runtime_state,
        style_source,
        maybe_spec,
        maybe_handle,
        mut bevy_style,
        maybe_border,
        maybe_background,
    ) in &mut query
    {
        let resolved = style_source.resolve(*runtime_state);
        *bevy_style = to_bevy_style(&resolved);
        let Some(next_spec) = render_material_spec_from_style(&resolved) else {
            continue;
        };
        if let Some(mut spec) = maybe_spec {
            *spec = next_spec.clone();
        }
        if let Some(mut border) = maybe_border {
            border.0 = style_color(
                resolved.border_color.as_deref(),
                Color::NONE,
                resolved.opacity,
            );
        }
        if maybe_background.is_some() {
            commands.entity(entity).remove::<BackgroundColor>();
        }
        let material = UiXmlEffectMaterial {
            color: next_spec.background.unwrap_or_else(|| {
                style_color(
                    resolved.background.as_deref(),
                    Color::NONE,
                    resolved.opacity,
                )
            }),
            border_color: next_spec.border_color.unwrap_or(Color::NONE),
            gradient_end: next_spec.gradient_end.unwrap_or(Color::NONE),
            radius: next_spec.radius_strength(),
            border_width: next_spec.border_width_strength(),
            gradient_mix: next_spec.gradient_end.map(|_| 1.0).unwrap_or(0.0),
            shadow_alpha: next_spec.shadow_alpha(),
        };
        if let Some(handle) = maybe_handle {
            if let Some(existing) = materials.get_mut(handle) {
                *existing = material;
            }
        } else {
            let handle = materials.add(material);
            commands.entity(entity).insert(handle);
        }
    }
}

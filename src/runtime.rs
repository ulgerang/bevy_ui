use crate::render_effects::outline_from_style;
use crate::style::{style_color, to_bevy_style, UiStyle};
use crate::ElementNode;
use bevy::input::keyboard::{KeyCode, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::window::ReceivedCharacter;
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
pub struct UiXmlTextInput;

#[derive(Component, Debug, Clone, PartialEq, Eq, Default)]
pub struct UiXmlTextValue(pub String);

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiXmlTextDisplay(pub Entity);

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UiXmlTextChanged {
    pub entity: Entity,
    pub scope: Entity,
    pub name: Option<String>,
    pub previous_value: String,
    pub value: String,
}

#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlFocus {
    pub entity: Option<Entity>,
}

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiXmlRuntimeState {
    pub hovered: bool,
    pub active: bool,
    pub disabled: bool,
    pub focused: bool,
    pub style_generation: u64,
}

#[derive(Component, Debug, Clone)]
pub struct UiXmlStyleSource {
    pub base: UiStyle,
    pub hover: UiStyle,
    pub active: UiStyle,
    pub focus: UiStyle,
    pub disabled: UiStyle,
}

impl UiXmlStyleSource {
    pub(crate) fn from_runtime_styles(
        base: &UiStyle,
        hover: &UiStyle,
        active: &UiStyle,
        focus: &UiStyle,
        disabled: &UiStyle,
    ) -> Self {
        Self {
            base: base.without_state_styles(),
            hover: state_overlay(hover, base.hover.as_deref()),
            active: state_overlay(active, base.active.as_deref()),
            focus: state_overlay(focus, base.focus.as_deref()),
            disabled: state_overlay(disabled, base.disabled.as_deref()),
        }
    }

    fn resolve(&self, runtime_state: UiXmlRuntimeState) -> UiStyle {
        let mut style = self.base.clone();
        if runtime_state.disabled {
            style.merge(&self.disabled);
            return style;
        }
        if runtime_state.focused {
            style.merge(&self.focus);
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
        }
    }

    pub(crate) fn from_runtime_styles(
        base: &UiStyle,
        hover: &UiStyle,
        active: &UiStyle,
        focus: &UiStyle,
        disabled: &UiStyle,
    ) -> Self {
        Self {
            base: base.without_state_styles(),
            hover: Some(state_overlay(hover, base.hover.as_deref())),
            active: Some(state_overlay(active, base.active.as_deref())),
            focus: Some(state_overlay(focus, base.focus.as_deref())),
            disabled: Some(state_overlay(disabled, base.disabled.as_deref())),
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

impl Plugin for UiXmlPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<UiXmlControlChanged>()
            .add_event::<UiXmlTextChanged>()
            .add_event::<ReceivedCharacter>()
            .add_event::<KeyboardInput>()
            .init_resource::<UiXmlStyleRuntime>()
            .init_resource::<UiXmlFocus>()
            .init_resource::<UiXmlSelectorContextCache>()
            .add_systems(
                Update,
                (
                    register_selector_contexts,
                    mark_style_runtime_generation,
                    normalize_initial_radio_groups,
                    focus_pressed_text_inputs,
                    apply_control_interactions,
                    apply_text_input,
                    sync_text_display,
                    sync_runtime_state,
                    apply_interaction_styles,
                )
                    .chain(),
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

type TextFocusInteractionQuery<'w, 's> = Query<
    'w,
    's,
    (Entity, &'static Interaction, &'static UiXmlDisabled),
    (With<UiXmlTextInput>, Changed<Interaction>),
>;

fn focus_pressed_text_inputs(
    mut focus: ResMut<UiXmlFocus>,
    query: TextFocusInteractionQuery<'_, '_>,
) {
    for (entity, interaction, disabled) in &query {
        if *interaction == Interaction::Pressed && !disabled.0 {
            focus.entity = Some(entity);
        }
    }
}

type TextInputMutationQuery<'w, 's> = Query<
    'w,
    's,
    (
        &'static UiXmlDisabled,
        &'static mut UiXmlTextValue,
        Option<&'static UiXmlControlScope>,
        Option<&'static UiXmlControlName>,
    ),
    With<UiXmlTextInput>,
>;

fn apply_text_input(
    focus: Res<UiXmlFocus>,
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

    let Ok((disabled, mut value, scope, name)) = query.get_mut(entity) else {
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
        if received.char.is_control() {
            continue;
        }
        let previous_value = value.0.clone();
        value.0.push(received.char);
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
        if input.key_code != Some(KeyCode::Back) {
            continue;
        }
        if value.0.is_empty() {
            continue;
        }
        let previous_value = value.0.clone();
        value.0.pop();
        send_text_changed(
            &mut events,
            entity,
            scope,
            name,
            previous_value,
            value.0.clone(),
        );
    }
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

type TextDisplaySyncQuery<'w, 's> =
    Query<'w, 's, (&'static UiXmlTextValue, &'static UiXmlTextDisplay), Changed<UiXmlTextValue>>;

fn sync_text_display(value_query: TextDisplaySyncQuery<'_, '_>, mut text_query: Query<&mut Text>) {
    for (value, display) in &value_query {
        let Ok(mut text) = text_query.get_mut(display.0) else {
            continue;
        };
        if let Some(section) = text.sections.first_mut() {
            section.value = value.0.clone();
        }
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

type RuntimeStateQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Option<&'static Interaction>,
        &'static UiXmlDisabled,
        &'static mut UiXmlRuntimeState,
    ),
>;

fn sync_runtime_state(focus: Res<UiXmlFocus>, mut query: RuntimeStateQuery<'_, '_>) {
    for (entity, interaction, disabled, mut state) in &mut query {
        let next_disabled = disabled.0;
        let next_focused = focus.entity == Some(entity) && !next_disabled;
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
            && state.active == next_active
            && state.hovered == next_hovered
        {
            continue;
        }

        state.disabled = next_disabled;
        state.focused = next_focused;
        state.active = next_active;
        state.hovered = next_hovered;
        if disabled.0 {
            continue;
        }
    }
}

type InteractionStyleQuery<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static UiXmlRuntimeState,
        &'static UiXmlStyleSource,
        &'static mut Style,
        &'static mut BackgroundColor,
        &'static mut BorderColor,
        Option<&'static mut Outline>,
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
        mut bevy_style,
        mut background,
        mut border,
        maybe_outline,
    ) in &mut query
    {
        let resolved = style_source.resolve(*runtime_state);
        *bevy_style = to_bevy_style(&resolved);
        background.0 = style_color(
            resolved.background.as_deref(),
            Color::NONE,
            resolved.opacity,
        );
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
    }
}

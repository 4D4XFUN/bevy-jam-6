use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(InputContext)]
pub struct PlayerActions;

pub fn plugin(app: &mut App) {
    app.add_plugins(EnhancedInputPlugin);

    app.add_input_context::<PlayerActions>();

    app.add_observer(regular_binding);
}

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct PlayerMoveAction;

#[derive(Debug, InputAction)]
#[input_action(output = bool)]
pub struct AimModeAction;

impl AimModeAction {
    const KEY: KeyCode = KeyCode::Space;
    const GAMEPAD: GamepadButton = GamepadButton::LeftTrigger;
    const DELAY_SECONDS: f32 = 0.3;
}
fn regular_binding(
    trigger: Trigger<Binding<PlayerActions>>,
    mut player: Query<&mut Actions<PlayerActions>>,
) {
    // We have to bind the input mapping to the player at runtime
    let mut actions = player.get_mut(trigger.target()).unwrap();
    actions
        .bind::<PlayerMoveAction>()
        .to((
            Cardinal::wasd_keys(),
            Axial::left_stick(),
            Cardinal::arrow_keys(),
            Cardinal::dpad_buttons(),
        ))
        .with_modifiers(DeadZone::default());

    actions
        .bind::<AimModeAction>()
        .to((AimModeAction::KEY, AimModeAction::GAMEPAD))
        .with_conditions(Hold::new(AimModeAction::DELAY_SECONDS)); // trigger after this many seconds
}

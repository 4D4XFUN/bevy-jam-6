use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

#[derive(Debug, InputAction)]
#[input_action(output = Vec2)]
pub struct PlayerMove;

#[derive(InputContext)]
pub struct PlayerActions;

pub fn plugin(app: &mut App) {
    app.add_plugins(EnhancedInputPlugin);

    app.add_input_context::<PlayerActions>();
    
    app.add_observer(regular_binding);
}

fn regular_binding(trigger: Trigger<Binding<PlayerActions>>, mut player: Query<&mut Actions<PlayerActions>>) {
    // We have to bind the input mapping to the player at runtime
    let mut actions = player.get_mut(trigger.target()).unwrap();
    actions
        .bind::<PlayerMove>()
        .to((Cardinal::wasd_keys(), Axial::left_stick()))
        .with_modifiers(DeadZone::default());
}
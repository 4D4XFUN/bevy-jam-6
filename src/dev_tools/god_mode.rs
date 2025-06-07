use crate::gameplay::health_and_damage::Health;
use crate::gameplay::player::Player;
use avian3d::prelude::RigidBody;
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.init_state::<GodModeState>();

    app.add_systems(
        Update,
        toggle_god_mode.run_if(input_just_pressed(KeyCode::KeyG)),
    );

    app.add_systems(OnEnter(GodModeState::God), enable_god_mode);
    app.add_systems(OnEnter(GodModeState::Normal), disable_god_mode);
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, States)]
pub enum GodModeState {
    Normal,
    #[default]
    God,
}
fn toggle_god_mode(
    curr_screen: Res<State<GodModeState>>,
    mut next_screen: ResMut<NextState<GodModeState>>,
) {
    let next = match curr_screen.get() {
        GodModeState::Normal => GodModeState::God,
        GodModeState::God => GodModeState::Normal,
    };
    info!("god mode: {:?}", next);
    next_screen.set(next);
}

fn enable_god_mode(player: Single<Entity, With<Player>>, mut commands: Commands) {
    commands
        .entity(player.into_inner())
        .insert(RigidBody::Kinematic)
        .insert(Health(9000));
}
fn disable_god_mode(player: Single<Entity, With<Player>>, mut commands: Commands) {
    commands
        .entity(player.into_inner())
        .insert(RigidBody::Dynamic)
        .insert(Health::default());
}

use crate::gameplay::health_and_damage::{DeathEvent, Health, HealthEvent};
use crate::gameplay::player::{MovementSettings, Player};
use avian3d::prelude::RigidBody;
use bevy::input::common_conditions::input_just_pressed;
use bevy::prelude::*;
use crate::gameplay::enemy::Enemy;

/// GOD MODE
/// press 'g' to enter/exit
/// 
/// While in it:
/// - 1 kills all enemies
/// - 2 kills player
pub fn plugin(app: &mut App) {
    app.init_state::<GodModeState>();

    app.add_systems(
        Update,
        toggle_god_mode.run_if(input_just_pressed(KeyCode::KeyG)),
    );

    app.add_systems(
        Update,
        kill_all_enemies
            .run_if(input_just_pressed(KeyCode::Digit1))
            .run_if(in_state(GodModeState::God)),
    );
    
    app.add_systems(
        Update,
        kill_player
            .run_if(input_just_pressed(KeyCode::Digit2))
            .run_if(in_state(GodModeState::God)),
    );

    app.add_systems(OnEnter(GodModeState::God), enable_god_mode);
    app.add_systems(OnEnter(GodModeState::Normal), disable_god_mode);
}

#[derive(Clone, Debug, Default, Eq, Hash, PartialEq, States)]
pub enum GodModeState {
    #[default]
    Normal,
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
        .insert(MovementSettings { walk_speed: 40. })
        .insert(Health(9000));
}
fn disable_god_mode(player: Single<Entity, With<Player>>, mut commands: Commands) {
    commands
        .entity(player.into_inner())
        .insert(RigidBody::Dynamic)
        .insert(MovementSettings::default())
        .insert(Health::default());
}

fn kill_all_enemies(enemies: Query<Entity, (With<Enemy>, With<Health>)>, mut commands: Commands) {
    info!("kill {} enemies:", enemies.iter().len());
    for e in enemies.iter() {
        commands.entity(e).trigger(HealthEvent::Damage(100, 1));
    }
}
fn kill_player(player: Single<Entity, (With<Player>, With<Health>)>, mut commands: Commands) {
    let p = player.into_inner();
    info!("kill player: {}", p);
    commands.entity(p).trigger(DeathEvent(1));
}

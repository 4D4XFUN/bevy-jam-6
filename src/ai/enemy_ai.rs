use crate::ai::pathfinding_service::PathfindingState;
use crate::gameplay::Gameplay;
use crate::gameplay::enemy::Enemy;
use crate::gameplay::player::Player;
use avian3d::prelude::LinearVelocity;
#[allow(unused_imports)]
use bevy::color::palettes;
use bevy::prelude::*;

pub fn plugin(app: &mut App) {
    app.register_type::<FollowPlayerBehavior>();
    app.add_plugins(AiMovementState::plugin);
}

/// Example usage:
/// ```
/// commands.spawn((
///     Enemy,
///     Transform::from_translation(...),
///     AiMovementBehavior::follow_player(),
/// ));
/// ```
#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct FollowPlayerBehavior {
    pub detection_range: f32,
    /// How close to get before we stop moving
    pub distance_to_keep: f32,
    /// If player moves this far, we'll recalculate our path
    pub staleness_range: f32,
    pub movement_speed: f32,
}
impl Default for FollowPlayerBehavior {
    fn default() -> Self {
        Self {
            distance_to_keep: 0.0,
            detection_range: 9000.0,
            staleness_range: 10.,
            movement_speed: 2.,
        }
    }
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub enum AiMovementState {
    Observing,
    FindingPath,
    Moving { path: Vec<Vec3>, index: usize },
}
impl AiMovementState {
    pub fn plugin(app: &mut App) {
        app.add_observer(
            |t: Trigger<OnAdd, FollowPlayerBehavior>, mut commands: Commands| {
                commands
                    .entity(t.target())
                    .insert(AiMovementState::Observing);
            },
        );
        app.add_systems(
            Update,
            Self::following_player_state_machine.run_if(in_state(Gameplay::Normal)),
        );
        app.register_type::<AiMovementState>();
    }

    fn following_player_state_machine(
        player: Single<&Transform, (With<Player>, Without<Enemy>)>,
        mut enemies: Query<
            (
                Entity,
                &mut Transform,
                &mut AiMovementState,
                &FollowPlayerBehavior,
                &mut LinearVelocity,
                Option<&PathfindingState>,
            ),
            (With<Enemy>, Without<Player>),
        >,
        mut commands: Commands,
        #[cfg(feature = "dev")] mut gizmos: Gizmos,
    ) {
        let target = player.translation;
        for (e, t, state, behavior, mut linear_velocity, pathfinding) in enemies.iter_mut() {
            let me = t.translation;
            let state = state.into_inner();
            match state {
                AiMovementState::Observing => {
                    if target.distance(me) < behavior.detection_range
                        && target.distance(me) > behavior.distance_to_keep
                    {
                        commands
                            .entity(e)
                            .insert(PathfindingState::new(t.translation, target))
                            .insert(AiMovementState::FindingPath);
                    }
                }
                AiMovementState::FindingPath => {
                    if let Some(PathfindingState::Completed(found_path)) = pathfinding {
                        commands
                            .entity(e)
                            .insert(AiMovementState::Moving {
                                index: 1,
                                path: found_path.clone(),
                            })
                            .remove::<PathfindingState>();
                    }
                }
                AiMovementState::Moving { path, index } => {
                    // first, a staleness check - if player has moved too far from the original path we want to recompute it instead.
                    let target_deviation = path.last().map(|v| v.distance(target)).unwrap_or(0.);
                    if target_deviation > behavior.staleness_range {
                        info!("target moved! recalculating...");
                        commands.entity(e).insert(AiMovementState::Observing);
                        continue;
                    }

                    let me = me.with_y(0.0); // our capsules' y are 1.0, while the pathfinding nodes are at 0.0
                    let next = path.get(*index).unwrap_or(&target);
                    let dist = (next - me).length();
                    let dir = (next - me).normalize_or_zero() * behavior.movement_speed;
                    linear_velocity.x = dir.x;
                    linear_velocity.z = dir.z;

                    // debug visualization
                    #[cfg(feature = "dev")]
                    gizmos.linestrip(
                        path.clone().iter().map(|v| v.with_y(0.2)),
                        palettes::css::BLUE,
                    );

                    // We've reached a waypoint, increment the index to the next one
                    if dist < 1. {
                        *index += 1; // doesn't work
                    }

                    if *index >= path.len() {
                        commands.entity(e).insert(AiMovementState::Observing);
                    }
                }
            }
        }
    }
}

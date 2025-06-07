use bevy::prelude::*;
use oxidized_navigation::query::{find_polygon_path, perform_string_pulling_on_path};
use oxidized_navigation::{NavMesh, NavMeshSettings};

use crate::gameplay::Gameplay;

pub fn plugin(app: &mut App) {
    app.init_resource::<PathfindingService>();
    app.add_systems(
        Update,
        PathfindingService::run_blocking_pathfinding.run_if(in_state(Gameplay::Normal)),
    );
}

#[derive(Component)]
pub enum PathfindingState {
    Requested { a: Vec3, b: Vec3 },
    Completed(Vec<Vec3>),
}
impl PathfindingState {
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self::Requested { a, b }
    }
}

#[derive(Default, Resource)]
struct PathfindingService;

impl PathfindingService {
    fn run_blocking_pathfinding(
        mut commands: Commands,
        query: Query<(Entity, &PathfindingState)>,
        nav_mesh_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
    ) {
        // Get the underlying nav_mesh.
        if let Ok(nav_mesh) = nav_mesh.get().read() {
            for (entity, state) in query.iter() {
                match state {
                    PathfindingState::Requested { a, b } => {
                        // execute the sync pathfinding job
                        let start_pos = *a;
                        let end_pos = *b;

                        // Run pathfinding to get a polygon path.
                        match find_polygon_path(
                            &nav_mesh,
                            &nav_mesh_settings,
                            start_pos,
                            end_pos,
                            None,
                            Some(&[1.0, 0.5]),
                        ) {
                            Ok(path) => {
                                // Convert polygon path to a path of Vec3s.
                                match perform_string_pulling_on_path(
                                    &nav_mesh, start_pos, end_pos, &path,
                                ) {
                                    Ok(string_path) => {
                                        commands
                                            .entity(entity)
                                            .insert(PathfindingState::Completed(string_path));
                                    }
                                    Err(error) => error!("Error with string path: {:?}", error),
                                };
                            }
                            Err(error) => error!("Error with pathfinding: {:?}", error),
                        }
                    }
                    _ => continue,
                }
            }
        }
    }
}

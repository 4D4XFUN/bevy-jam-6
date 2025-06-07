use crate::gameplay::enemy::Enemy;
use crate::gameplay::player::Player;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{AsyncComputeTaskPool, Task, block_on};
use oxidized_navigation::debug_draw::DrawPath;
use oxidized_navigation::query::{find_path, find_polygon_path, perform_string_pulling_on_path};
use oxidized_navigation::tiles::NavMeshTiles;
use oxidized_navigation::{NavMesh, NavMeshSettings};
use rand::{Rng, random};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::task::Poll;

pub fn plugin(app: &mut App) {
    app.init_resource::<PathfindingService>();
    app.add_systems(
        Update,
        (
            PathfindingService::run_async_pathfinding,
            PathfindingService::poll_running_pathfinding_tasks,
        )
            .chain(),
    );
}

#[derive(Component)]
pub enum PathfindingState {
    Requested { a: Vec3, b: Vec3 },
    Pending(TaskId),
    Completed(Vec<Vec3>),
}
impl PathfindingState {
    pub fn new(a: Vec3, b: Vec3) -> Self {
        Self::Requested {
            a,
            b,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct TaskId(usize);
impl TaskId {
    pub fn new() -> Self {
        Self(random())
    }
}

#[derive(Default, Resource)]
struct PathfindingService {
    tasks: HashMap<TaskId, Task<Option<Vec<Vec3>>>>,
}

impl PathfindingService {
    // Queue up pathfinding tasks.
    fn run_async_pathfinding(
        mut commands: Commands,
        query: Query<(Entity, &PathfindingState)>,
        nav_mesh_settings: Res<NavMeshSettings>,
        nav_mesh: Res<NavMesh>,
        mut pathfinding_task: ResMut<PathfindingService>,
    ) {
        for (entity, state) in query.iter() {
            match state {
                PathfindingState::Requested { a, b } => {
                    // execute the async pathfinding job
                    let thread_pool = AsyncComputeTaskPool::get();
                    let nav_mesh_lock = nav_mesh.get();
                    let task = thread_pool.spawn(Self::async_path_find(
                        nav_mesh_lock,
                        nav_mesh_settings.clone(),
                        *a,
                        *b,
                        None,
                    ));

                    // keep track of the job in a new component on the same entity
                    let id = TaskId::new();
                    pathfinding_task.tasks.insert(id, task);
                    commands
                        .entity(entity)
                        .insert(PathfindingState::Pending(id));
                }
                _ => continue,
            }
        }
    }

    /// Async wrapper function for path finding.
    async fn async_path_find(
        nav_mesh_lock: Arc<RwLock<NavMeshTiles>>,
        nav_mesh_settings: NavMeshSettings,
        start_pos: Vec3,
        end_pos: Vec3,
        position_search_radius: Option<f32>,
    ) -> Option<Vec<Vec3>> {
        // Get the underlying nav_mesh.
        let Ok(nav_mesh) = nav_mesh_lock.read() else {
            return None;
        };

        // Run pathfinding to get a path.
        match find_path(
            &nav_mesh,
            &nav_mesh_settings,
            start_pos,
            end_pos,
            position_search_radius,
            Some(&[1.0, 0.5]),
        ) {
            Ok(path) => {
                info!("Found path (ASYNC): {:?}", path);
                return Some(path);
            }
            Err(error) => error!("Error with pathfinding: {:?}", error),
        }

        None
    }

    fn poll_running_pathfinding_tasks(
        mut commands: Commands,
        query: Query<(Entity, &PathfindingState)>,
        mut pathfinding_task: ResMut<PathfindingService>,
    ) {
        for (entity, state) in query.iter() {
            match state {
                PathfindingState::Pending(id) => {
                    if let Some(t) = pathfinding_task.tasks.get_mut(&id) {
                        if let Some(string_path) = block_on(future::poll_once(t)).unwrap_or(None) {
                            pathfinding_task.tasks.remove(&id);
                            commands
                                .entity(entity)
                                .insert(PathfindingState::Completed(string_path));
                        }
                    }
                }
                _ => continue,
            }
        }
    }
}

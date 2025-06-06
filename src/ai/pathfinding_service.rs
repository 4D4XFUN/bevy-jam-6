use crate::gameplay::enemy::Enemy;
use crate::gameplay::player::Player;
use bevy::color::palettes;
use bevy::prelude::*;
use bevy::tasks::futures_lite::future;
use bevy::tasks::{AsyncComputeTaskPool, Task};
use oxidized_navigation::debug_draw::DrawPath;
use oxidized_navigation::query::{find_path, find_polygon_path, perform_string_pulling_on_path};
use oxidized_navigation::tiles::NavMeshTiles;
use oxidized_navigation::{NavMesh, NavMeshSettings};
use rand::{Rng, random};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub fn plugin(app: &mut App) {
    app.init_resource::<AsyncPathfindingTasks>();
    app.add_systems(
        Update,
        (run_async_pathfinding, poll_pathfinding_tasks_system),
    );
}

/// Provides a single entrypoint to pathfinding along the level's navmesh. Under
/// the hood, it asynchronously computes paths and updates entities once the
/// tasks complete.
#[derive(Default)]
pub struct PathfindingService {
    tasks: HashMap<TaskId, Option<Task<Vec<Vec3>>>>,
}
impl PathfindingService {
    pub fn start_pathfinding(from: Vec3, to: Vec3) -> TaskId {
        TaskId::new()
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct TaskId(usize);
impl TaskId {
    pub fn new() -> Self {
        Self(random())
    }
}

//  Running pathfinding in a task without blocking the frame.
//  Also check out Bevy's async compute example.
//  https://github.com/bevyengine/bevy/blob/main/examples/async_tasks/async_compute.rs

// Holder resource for tasks.
#[derive(Default, Resource)]
struct AsyncPathfindingTasks {
    tasks: Vec<Task<Option<Vec<Vec3>>>>,
}
// Queue up pathfinding tasks.
fn run_async_pathfinding(
    keys: Res<ButtonInput<KeyCode>>,
    nav_mesh_settings: Res<NavMeshSettings>,
    nav_mesh: Res<NavMesh>,
    mut pathfinding_task: ResMut<AsyncPathfindingTasks>,
) {
    if !keys.just_pressed(KeyCode::KeyA) {
        return;
    }

    let thread_pool = AsyncComputeTaskPool::get();

    let nav_mesh_lock = nav_mesh.get();
    let start_pos = Vec3::new(5.0, 1.0, 5.0);
    let end_pos = Vec3::new(-15.0, 1.0, -15.0);

    let task = thread_pool.spawn(async_path_find(
        nav_mesh_lock,
        nav_mesh_settings.clone(),
        start_pos,
        end_pos,
        None,
    ));

    pathfinding_task.tasks.push(task);
}

// Poll existing tasks.

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

fn poll_pathfinding_tasks_system(
    mut commands: Commands,
    mut pathfinding_task: ResMut<AsyncPathfindingTasks>,
) {
    // Go through and remove completed tasks.
    pathfinding_task.tasks.retain_mut(|task| {
        if let Some(string_path) = future::block_on(future::poll_once(task)).unwrap_or(None) {
            info!("Async path task finished with result: {:?}", string_path);
            commands.spawn(DrawPath {
                timer: Some(Timer::from_seconds(4.0, TimerMode::Once)),
                pulled_path: string_path,
                color: palettes::css::BLUE.into(),
            });

            false
        } else {
            true
        }
    });
}

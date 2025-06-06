use bevy::color::palettes;
use bevy::prelude::*;
use oxidized_navigation::{NavMesh, NavMeshSettings};
use oxidized_navigation::debug_draw::DrawPath;
use oxidized_navigation::query::{find_polygon_path, perform_string_pulling_on_path};
use crate::gameplay::enemy::Enemy;
use crate::gameplay::player::Player;

pub fn plugin(app: &mut App) {
    // todo
    app.add_systems(Update, (run_blocking_pathfinding,));
}

fn run_blocking_pathfinding(
    enemies: Query<&Transform, (With<Enemy>, Without<Player>)>,
    player: Single<&Transform, (With<Player>, Without<Enemy>)>,
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    nav_mesh_settings: Res<NavMeshSettings>,
    nav_mesh: Res<NavMesh>,
) {
    // if !keys.just_pressed(KeyCode::KeyB) {
    //    return;
    // }

    // Get the underlying nav_mesh.
    if let Ok(nav_mesh) = nav_mesh.get().read() {
        let start_pos = player.translation;
        let end_pos = enemies.iter().next().unwrap().translation;

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
                info!("Path found (BLOCKING): {:?}", path);

                // Convert polygon path to a path of Vec3s.
                match perform_string_pulling_on_path(&nav_mesh, start_pos, end_pos, &path) {
                    Ok(string_path) => {
                        info!("String path (BLOCKING): {:?}", string_path);
                        commands.spawn(DrawPath {
                            timer: Some(Timer::from_seconds(1.0/30., TimerMode::Once)),
                            pulled_path: string_path,
                            color: palettes::css::CHARTREUSE.into(),
                        });
                    }
                    Err(error) => error!("Error with string path: {:?}", error),
                };
            }
            Err(error) => error!("Error with pathfinding: {:?}", error),
        }
    }
}

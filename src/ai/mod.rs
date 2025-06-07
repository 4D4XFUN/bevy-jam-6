mod debug;
pub mod enemy_ai;
pub mod navmesh_position;
pub mod pathfinding_service;

use bevy::ecs::error::info;
use bevy::prelude::*;
use bevy::prelude::*;
use bevy_landmass::prelude::*;
use landmass::{AgentId, Archipelago, IslandId, PointSampleDistance3d, XYZ};
use landmass_oxidized_navigation::{LandmassOxidizedNavigationPlugin, OxidizedArchipelago};
use oxidized_navigation::{
    NavMesh, NavMeshSettings, OxidizedNavigationPlugin, colliders::avian::AvianCollider,
};

pub fn plugin(app: &mut App) {
    // plugins
    app.add_plugins((
        // navmesh_position::plugin,
        pathfinding_service::plugin,
        enemy_ai::plugin,
        debug::plugin,
        // Landmass3dPlugin::default(),
        // LandmassOxidizedNavigationPlugin::default(),
        OxidizedNavigationPlugin::<AvianCollider>::new(NavMeshSettings::from_agent_and_bounds(
            1.1, 1.9, 250.0, -1.0,
        )),
    ));

    // systems
    app.add_systems(Startup, setup_archipelago);
}

// Component to mark the player character
#[derive(Component)]
struct Character;

// Component to store the computed path
#[derive(Component)]
struct PathAgent {
    path: Vec<Vec3>,
    current_waypoint: usize,
}

// Resource to store the pathfinding archipelago
#[derive(Resource)]
struct PathfindingArchipelago {
    archipelago: Archipelago<ThreeD>,
}

fn setup_archipelago(mut commands: Commands) {
    // This *should* be scoped to the `Screen::Gameplay` state, but doing so
    // seems to never regenerate the nav mesh when the level is loaded the second
    // time.
    info!("Spawning archipelago");
    commands.spawn((
        Name::new("Main Level Archipelago"),
        Archipelago3d::new(AgentOptions {
            point_sample_distance: PointSampleDistance3d {
                horizontal_distance: 0.6,
                distance_above: 1.0,
                distance_below: 1.0,
                vertical_preference_ratio: 2.0,
            },
            ..AgentOptions::from_agent_radius(0.5)
        }),
        OxidizedArchipelago,
    ));
}

/*
fn setup_landmass(
    mut commands: Commands,
    nav_mesh: Res<NavMesh>, // Assuming you have your oxidized_navigation NavMesh as a resource
) {
    // Create the archipelago (collection of islands/navmeshes)
    let mut archipelago = Archipelago::new(AgentOptions::from_agent_radius(0.5));

    // Convert the oxidized_navigation NavMesh to a landmass-compatible format
    // let landmass_nav_mesh =

    // Create an island from the navmesh
    // let island = Island::new(
    //     landmass_nav_mesh.vertices.clone(),
    //     landmass_nav_mesh.polygons.clone(),
    //     landmass_nav_mesh.polygon_type_indices.clone(),
    // );

    // Add the island to the archipelago
    let island_id = archipelago.add_island(island);

    // Store the archipelago as a resource
    commands.insert_resource(PathfindingArchipelago { archipelago });

    // Store the island ID for later use (you might want to make this a resource too)
    commands.insert_resource(CurrentIslandId(island_id));
}

#[derive(Resource)]
struct CurrentIslandId(IslandId);

// ===============
// AGENTS
// ===============
mod agent {
    use std::f32::consts::TAU;

    use avian3d::prelude::*;
    use bevy::prelude::*;
    use bevy_landmass::{prelude::*, TargetReachedCondition};

    use crate::sssssss
        gameplay::player::navmesh_position::LastValidPlayerNavmeshPosition, screens::Screen,
    };

    use super::{Npc, NPC_FLOAT_HEIGHT, NPC_RADIUS};

    pub(crate) const NPC_MAX_SLOPE: f32 = TAU / 6.0;

    pub(super) fn plugin(app: &mut App) {
        app.register_type::<Agent>();
        app.register_type::<AgentOf>();
        app.register_type::<WantsToFollowPlayer>();
        app.add_systems(
            RunFixedMainLoop,
            (sync_agent_velocity, set_controller_velocity)
                .chain()
                .in_set(RunFixedMainLoopSystem::BeforeFixedMainLoop)
                .before(LandmassSystemSet::SyncExistence)
                .run_if(in_state(Screen::Gameplay)),
        );
        app.add_systems(
            RunFixedMainLoop,
            update_agent_target.in_set(PrePhysicsAppSystems::UpdateNavmeshTargets),
        );
        app.add_observer(setup_npc_agent);
    }

    /// Setup the NPC agent. An "agent" is what `bevy_landmass` can move around.
    /// Since we use a floating character controller, we need to offset the agent's position by the character's float height.
    #[cfg_attr(feature = "hot_patch", hot)]
    fn setup_npc_agent(
        trigger: Trigger<OnAdd, Npc>,
        mut commands: Commands,
        archipelago: Single<Entity, With<Archipelago3d>>,
    ) {
        let npc = trigger.target();
        commands.spawn((
            Name::new("NPC Agent"),
            Transform::from_translation(Vec3::new(0.0, -NPC_FLOAT_HEIGHT, 0.0)),
            Agent3dBundle {
                agent: default(),
                settings: AgentSettings {
                    radius: NPC_RADIUS,
                    desired_speed: 7.0,
                    max_speed: 8.0,
                },
                archipelago_ref: ArchipelagoRef3d::new(*archipelago),
            },
            TargetReachedCondition::Distance(Some(2.0)),
            ChildOf(npc),
            AgentOf(npc),
            AgentTarget3d::default(),
            WantsToFollowPlayer,
        ));
    }
}


 */

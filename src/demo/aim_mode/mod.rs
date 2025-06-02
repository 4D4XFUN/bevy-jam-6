mod targeting;

use crate::asset_tracking::LoadResource;
use crate::audio::sound_effect;
use crate::demo::input::AimModeAction;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;

pub fn plugin(app: &mut App) {
    app.add_plugins(targeting::plugin);

    app.init_state::<AimModeState>();
    app.add_observer(enter_aim_mode).add_observer(exit_aim_mode);

    // sound effect!
    app.load_resource::<AimModeAssets>()
        .add_systems(OnEnter(AimModeState::Aiming), play_aim_mode_sound_effect);
}

// =====================
// STATE MACHINE
// =====================
#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Default)]
#[states(scoped_entities)]
pub enum AimModeState {
    #[default]
    Normal,
    Aiming,
}

fn enter_aim_mode(
    trigger: Trigger<Fired<AimModeAction>>,
    state: Res<State<AimModeState>>,
    mut next_state: ResMut<NextState<AimModeState>>,
) {
    // don't enter aim mode if we're already in it
    if state.get() == &AimModeState::Aiming {
        return;
    }

    info!("Entering aim mode");
    next_state.set(AimModeState::Aiming);
}

fn exit_aim_mode(
    trigger: Trigger<Completed<AimModeAction>>,
    state: Res<State<AimModeState>>,
    mut next_state: ResMut<NextState<AimModeState>>,
) {
    // we can only exit aim mode if we're in it
    if state.get() != &AimModeState::Aiming {
        return;
    }

    info!("Exiting aim mode");
    next_state.set(AimModeState::Normal);
}

// =====================
// AUDIO
// =====================
#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct AimModeAssets {
    #[dependency]
    enter: Handle<AudioSource>,
}

impl FromWorld for AimModeAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            enter: assets.load("audio/sound_effects/step1.ogg"),
        }
    }
}

fn play_aim_mode_sound_effect(mut commands: Commands, assets: Option<Res<AimModeAssets>>) {
    let Some(assets) = assets else {
        return;
    };
    commands.spawn(sound_effect(assets.enter.clone()));
}

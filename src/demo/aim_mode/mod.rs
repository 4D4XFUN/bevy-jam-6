mod targeting;

use crate::asset_tracking::LoadResource;
use crate::audio::sound_effect;
use crate::demo::input::AimModeAction;
use bevy::prelude::*;
use bevy_enhanced_input::prelude::*;
use rand::Rng;

pub fn plugin(app: &mut App) {
    app.add_plugins(targeting::plugin);

    app.init_state::<AimModeState>();
    app.add_observer(enter_aim_mode).add_observer(exit_aim_mode);

    // sound effect!
    app.load_resource::<AimModeAssets>()
        .add_systems(OnEnter(AimModeState::Aiming), play_aim_mode_sound_effect);
    app.add_observer(play_enemy_targeted_sound_effect);
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
    _trigger: Trigger<Fired<AimModeAction>>,
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
    _trigger: Trigger<Completed<AimModeAction>>,
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
    entering_aim_mode: Handle<AudioSource>,
    #[dependency]
    targeting1: Handle<AudioSource>,
    #[dependency]
    targeting2: Handle<AudioSource>,
    #[dependency]
    targeting3: Handle<AudioSource>,
    #[dependency]
    targeting4: Handle<AudioSource>,
    #[dependency]
    targeting5: Handle<AudioSource>,
}

impl FromWorld for AimModeAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            entering_aim_mode: assets
                .load("audio/sound_effects/571273__princeofworms__hawkeagle-cry-distant.ogg"),
            
            targeting1: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting2: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting3: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting4: assets.load("audio/sound_effects/spurs/spur1.ogg"),
            targeting5: assets.load("audio/sound_effects/spurs/spur1.ogg"),
        }
    }
}

fn play_aim_mode_sound_effect(mut commands: Commands, assets: Option<Res<AimModeAssets>>) {
    let Some(assets) = assets else {
        return;
    };
    commands.spawn(sound_effect(assets.entering_aim_mode.clone()));
}

#[derive(Event)]
pub struct PlayEnemyTargetedSound;

fn play_enemy_targeted_sound_effect(
    _trigger: Trigger<PlayEnemyTargetedSound>,
    mut commands: Commands,
    assets: Option<Res<AimModeAssets>>,
) {
    let Some(assets) = assets else {
        return;
    };

    let random_index = rand::thread_rng().gen_range(1..=5);

    let sound_asset = match random_index {
        1 => assets.targeting1.clone(),
        2 => assets.targeting2.clone(),
        3 => assets.targeting3.clone(),
        4 => assets.targeting4.clone(),
        5 => assets.targeting5.clone(),
        _ => unreachable!(),
    };

    commands.spawn(sound_effect(sound_asset));
}

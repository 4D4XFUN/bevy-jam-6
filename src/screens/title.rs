//! The title screen that appears when the game starts.

use bevy::prelude::*;

use crate::audio::music;
use crate::gameplay::level::LevelAssets;
use crate::ui_assets::{FontAssets, PanelAssets};
use crate::{asset_tracking::LoadResource, screens::Screen, theme::prelude::*};

pub(super) fn plugin(app: &mut App) {
    app.register_type::<PanelAssets>()
        .load_resource::<PanelAssets>()
        .register_type::<FontAssets>()
        .load_resource::<FontAssets>()
        .add_systems(
            OnEnter(Screen::Title),
            (spawn_title_screen, start_credits_music),
        );
}

fn spawn_title_screen(panel: Res<PanelAssets>, fonts: Res<FontAssets>, mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Title Screen"),
        StateScoped(Screen::Title),
        #[cfg(not(target_family = "wasm"))]
        children![
            widget::label_with_font("4D4XFUN Presents", &fonts.header),
            widget::header_with_font("A FISTFUL OF BOOMERANGS", &fonts.header),
            widget::header(""),
            widget::paneled_button("Play", enter_gameplay_screen, &panel, &fonts.header),
            widget::paneled_button("Credits", enter_credits_screen, &panel, &fonts.header),
            widget::paneled_button("Exit", exit_app, &panel, &fonts.header),
        ],
        #[cfg(target_family = "wasm")]
        children![
            widget::label_with_font("4D4XFUN Presents", &fonts.header),
            widget::header_with_font("A FISTFUL OF BOOMERANGS", &fonts.header),
            widget::header(""),
            widget::paneled_button("Play", enter_gameplay_screen, &panel, &fonts.header),
            widget::paneled_button("Credits", enter_credits_screen, &panel, &fonts.header),
        ],
    ));
}

fn start_credits_music(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        Name::new("Title Music"),
        StateScoped(Screen::Title),
        music(assets.load("audio/music/EcstasyOfSka.ogg")),
    ));
}

fn enter_gameplay_screen(
    _: Trigger<Pointer<Click>>,
    mut level_assets: ResMut<LevelAssets>,
    mut next_screen: ResMut<NextState<Screen>>,
) {
    level_assets.all_bounties.clear();
    next_screen.set(Screen::Gameplay);
}

fn _enter_settings_screen(_: Trigger<Pointer<Click>>, mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Settings);
}

fn enter_credits_screen(_: Trigger<Pointer<Click>>, mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Credits);
}
#[cfg(not(target_family = "wasm"))]
fn exit_app(_: Trigger<Pointer<Click>>, mut app_exit: EventWriter<AppExit>) {
    app_exit.write(AppExit::Success);
}

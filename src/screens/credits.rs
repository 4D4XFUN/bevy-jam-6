//! A credits screen that can be accessed from the title screen.

use bevy::{ecs::spawn::SpawnIter, prelude::*, ui::Val::*};

use crate::gameplay::level::LevelAssets;
use crate::ui_assets::{FontAssets, PanelAssets};
use crate::{asset_tracking::LoadResource, audio::music, screens::Screen, theme::prelude::*};

pub(super) fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Screen::Credits), spawn_credits_screen);

    app.register_type::<CreditsAssets>();
    app.load_resource::<CreditsAssets>();
    app.add_systems(OnEnter(Screen::Credits), start_credits_music);
}

fn spawn_credits_screen(
    panel: Res<PanelAssets>,
    level_assets: Res<LevelAssets>,
    fonts: Res<FontAssets>,
    mut commands: Commands,
) {
    commands
        .spawn((
            widget::ui_root("Credits Screen"),
            StateScoped(Screen::Credits),
        ))
        .with_children(|parent| {
            if !level_assets.all_bounties.is_empty() {
                let bounty = level_assets.all_bounties.values().sum::<f32>();
                parent.spawn(widget::header_with_font(
                    format!("You collected $ {} in bounty total!", bounty),
                    &fonts.content,
                ));
            }
            parent.spawn(widget::header_with_font("Created by", &fonts.header));
            parent.spawn(created_by());
            parent.spawn(widget::header_with_font("Assets", &fonts.header));
            parent.spawn(assets());
            parent.spawn(widget::paneled_button(
                "Back",
                enter_title_screen,
                &panel,
                &fonts.header,
            ));
        });
}

fn created_by() -> impl Bundle {
    grid(vec![
        ["Emily 'tigerplush' P.", "UI, SFX"],
        ["Sam 'sfarmer1'", "Design, Programming"],
        ["Martin 'mpwoz'", "Programming"],
        ["Jacudibu", "Programming"],
        ["BurnteToaster", "SFX"],
    ])
}

fn assets() -> impl Bundle {
    grid(vec![["Pistol Ricochet Sound", "CC0 by Diboz"]])
}

fn grid(content: Vec<[&'static str; 2]>) -> impl Bundle {
    (
        Name::new("Grid"),
        Node {
            display: Display::Grid,
            row_gap: Px(10.0),
            column_gap: Px(30.0),
            grid_template_columns: RepeatedGridTrack::px(2, 400.0),
            ..default()
        },
        Children::spawn(SpawnIter(content.into_iter().flatten().enumerate().map(
            |(i, text)| {
                (
                    widget::label(text),
                    Node {
                        justify_self: if i % 2 == 0 {
                            JustifySelf::End
                        } else {
                            JustifySelf::Start
                        },
                        ..default()
                    },
                )
            },
        ))),
    )
}

fn enter_title_screen(_: Trigger<Pointer<Click>>, mut next_screen: ResMut<NextState<Screen>>) {
    next_screen.set(Screen::Title);
}

#[derive(Resource, Asset, Clone, Reflect)]
#[reflect(Resource)]
struct CreditsAssets {
    #[dependency]
    music: Handle<AudioSource>,
}

impl FromWorld for CreditsAssets {
    fn from_world(world: &mut World) -> Self {
        let assets = world.resource::<AssetServer>();
        Self {
            music: assets.load("audio/music/EcstasyOfSka.ogg"),
        }
    }
}

fn start_credits_music(mut commands: Commands, credits_music: Res<CreditsAssets>) {
    commands.spawn((
        Name::new("Credits Music"),
        StateScoped(Screen::Credits),
        music(credits_music.music.clone()),
    ));
}

use avian3d::prelude::Physics;
use bevy::{
    color::palettes::css::{BLACK, WHITE},
    prelude::*,
};

use crate::theme::film_grain::{
    FilmGrainSettings, FilmGrainSettingsPresets, FilmGrainSettingsTween,
};
use crate::{
    gameplay::Gameplay,
    screens::Screen,
    theme::widget,
    ui_assets::{FontAssets, PanelAssets},
};

pub fn plugin(app: &mut App) {
    app.register_type::<Score>()
        .add_systems(OnEnter(Gameplay::GameOver), (setup, close_vignette_on_death))
        .add_systems(OnEnter(Screen::Retry), retry)
        .add_systems(OnEnter(Gameplay::Normal), (setup_scoreboard, tween_to_default_camera_settings))
        .add_systems(
            Update,
            update_score.run_if(in_state(Screen::Gameplay).and(resource_changed::<Score>)),
        )
        .add_observer(on_score_event);
}

fn setup(panel: Res<PanelAssets>, font_assets: Res<FontAssets>, mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Game Over Screen"),
        StateScoped(Gameplay::GameOver),
        BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
        children![
            widget::label_with_font(
                "You been took t' an early grave, pardner",
                &font_assets.header
            ),
            widget::paneled_button("Retry", retry_level, &panel, &font_assets.header),
            widget::paneled_button("Main Menu", main_menu, &panel, &font_assets.header),
        ],
    ));
}

fn close_vignette_on_death(
    camera: Single<Entity, (With<Camera>, With<FilmGrainSettings>)>,
    mut commands: Commands,
) {
    commands
        .entity(camera.into_inner())
        .insert(FilmGrainSettingsTween::new(
            2.,
            EaseFunction::CircularIn,
            FilmGrainSettingsPresets::VignetteClosed,
        ));
}

fn tween_to_default_camera_settings(
    camera: Single<Entity, (With<Camera>, With<FilmGrainSettings>)>,
    mut commands: Commands,
) {
    commands
        .entity(camera.into_inner())
        .insert(FilmGrainSettingsTween::new(
            0.5,
            EaseFunction::CircularIn,
            FilmGrainSettingsPresets::Default,
        ));
}

fn retry_level(_trigger: Trigger<Pointer<Click>>, mut next_state: ResMut<NextState<Screen>>) {
    next_state.set(Screen::Retry);
}

fn main_menu(_trigger: Trigger<Pointer<Click>>, mut next_state: ResMut<NextState<Screen>>) {
    next_state.set(Screen::Title);
}

fn retry(mut next_state: ResMut<NextState<Screen>>) {
    next_state.set(Screen::Gameplay);
}

#[derive(Component)]
struct ScoreBoard;

fn setup_scoreboard(font_assets: Res<FontAssets>, mut commands: Commands) {
    commands.spawn((
        Name::new("Scoreboard"),
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Start,
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(20.0),
            ..default()
        },
        // Don't block picking events for other UI roots.
        Pickable::IGNORE,
        StateScoped(Gameplay::Normal),
        children![(
            Text::new(""),
            TextFont {
                font: font_assets.content.clone(),
                font_size: 40.0,
                ..default()
            },
            TextColor(BLACK.into()),
            TextShadow {
                color: WHITE.into(),
                ..default()
            },
            ScoreBoard,
        )],
    ));

    commands.insert_resource(Score::default());
}

fn update_score(
    time: Res<Time<Physics>>,
    mut score: ResMut<Score>,
    scoreboard: Single<&mut Text, With<ScoreBoard>>,
) {
    score.current_t = (score.current_t + time.delta_secs()).clamp(0.0, 1.0);
    let mut text = scoreboard.into_inner();
    let current_score = score
        .old_score
        .lerp(score.actual_score, score.current_t)
        .ceil();
    text.0 = format!("$ {:05}", current_score);
    score.current_displayed_score = current_score;
}

fn on_score_event(trigger: Trigger<ScoreEvent>, mut score: ResMut<Score>) {
    score.current_t = 0.0;
    score.actual_score += trigger.event().0;
    score.old_score = score.current_displayed_score;
}

#[derive(Event)]
pub struct ScoreEvent(pub f32);

#[derive(Default, Reflect, Resource)]
#[reflect(Resource)]
pub struct Score {
    actual_score: f32,
    current_displayed_score: f32,
    old_score: f32,
    current_t: f32,
}

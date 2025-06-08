use avian3d::prelude::Physics;
use bevy::{
    color::palettes::css::{BLACK, WHITE},
    prelude::*,
};

use crate::audio::sound_effect_non_dilated;
use crate::gameplay::level::LevelAssets;
use crate::theme::film_grain::FilmGrainSettingsTween;
use crate::{
    gameplay::{Gameplay, enemy::Enemy, health_and_damage::Health},
    screens::Screen,
    theme::widget,
    ui_assets::{FontAssets, PanelAssets},
};

#[derive(Reflect, Resource)]
struct ScoreSettings {
    floating_score_speed: f32,
    min_font_size: f32,
    max_font_size: f32,
    /// font size reaches it's max after hitting a score of
    /// max_font_size_score in $
    max_font_size_score: f32,
    floating_score_fadeout_speed: f32,
}

impl Default for ScoreSettings {
    fn default() -> Self {
        ScoreSettings {
            floating_score_speed: 100.0,
            min_font_size: 20.0,
            max_font_size: 24.0,
            max_font_size_score: 1000.0,
            floating_score_fadeout_speed: 1.0,
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<Winner>()
        .init_resource::<ScoreSettings>();
    app.register_type::<Score>()
        .add_systems(
            OnEnter(Gameplay::GameOver),
            (
                setup,
                FilmGrainSettingsTween::tween_close_vignette_to_black_screen,
            ),
        )
        .add_systems(OnEnter(Screen::Retry), reload_current_level)
        .add_systems(OnEnter(Screen::NextLevel), load_next_level)
        .add_systems(
            OnEnter(Gameplay::Normal),
            (
                setup_scoreboard,
                FilmGrainSettingsTween::tween_to_default_camera_settings,
            ),
        )
        .add_systems(
            Update,
            update_score.run_if(in_state(Screen::Gameplay).and(resource_changed::<Score>)),
        )
        .add_systems(Update, float_score)
        .add_observer(on_score_event);
}

fn setup(
    panel: Res<PanelAssets>,
    score: Res<Score>,
    winner: Res<Winner>,
    level_assets: ResMut<LevelAssets>,
    font_assets: Res<FontAssets>,
    mut commands: Commands,
) {
    let text = match *winner {
        Winner::Player => {
            let level_data = level_assets.into_inner();
            level_data
                .all_bounties
                .entry(level_data.current_level)
                .and_modify(|entry| {
                    if *entry < score.actual_score {
                        *entry = score.actual_score
                    }
                })
                .or_insert(score.actual_score);
            info!("{:?}", level_data.all_bounties);
            format!("You claimed $ {} as bounty", score.actual_score)
        }
        Winner::Enemy => "You been took t' an early grave, pardner".to_string(),
    };
    commands
        .spawn((
            widget::ui_root("Game Over Screen"),
            StateScoped(Gameplay::GameOver),
            BackgroundColor(Color::srgba(0., 0., 0., 0.7)),
        ))
        .with_children(|parent| {
            parent.spawn((
                Name::new("Label"),
                Text::new("CONGRATS, COWBOY"),
                TextFont::from_font_size(40.0).with_font(font_assets.header.clone()),
            ));
            parent.spawn((
                Name::new("Label"),
                Text(text),
                TextFont::from_font_size(24.0).with_font(font_assets.content.clone()),
            ));
            if Winner::Player == *winner {
                parent.spawn(widget::paneled_button(
                    "Onward",
                    on_click_next_level,
                    &panel,
                    &font_assets.header,
                ));
            }
            parent.spawn(widget::paneled_button(
                "Retry",
                on_click_retry_level,
                &panel,
                &font_assets.header,
            ));
            parent.spawn(widget::paneled_button(
                "Main Menu",
                on_click_main_menu,
                &panel,
                &font_assets.header,
            ));
        });
}

fn on_click_retry_level(
    _trigger: Trigger<Pointer<Click>>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    next_state.set(Screen::Retry);
}

fn on_click_next_level(
    _trigger: Trigger<Pointer<Click>>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    next_state.set(Screen::NextLevel);
}

fn on_click_main_menu(
    _trigger: Trigger<Pointer<Click>>,
    mut next_state: ResMut<NextState<Screen>>,
) {
    next_state.set(Screen::Title);
}

fn reload_current_level(
    mut next_state: ResMut<NextState<Screen>>,
    level_assets: ResMut<LevelAssets>,
) {
    let level_data = level_assets.into_inner();
    info!("Restarting level {}", level_data.current_level);
    next_state.set(Screen::Gameplay);
}

fn load_next_level(
    mut next_state: ResMut<NextState<Screen>>,
    level_assets: ResMut<LevelAssets>,
    mut commands: Commands,
) {
    let level_data = level_assets.into_inner();
    if level_data.current_level < level_data.levels.len() - 1 {
        level_data.current_level += 1;
        info!("Loading next level: {}", level_data.current_level);
        next_state.set(Screen::Gameplay);

        // bird cry on start of last level
        if level_data.current_level == level_data.levels.len() - 1 {
            commands.spawn(sound_effect_non_dilated(level_data.eagle_sfx.clone(), 0.0));
        }
    } else {
        level_data.current_level = 0;
        next_state.set(Screen::Credits);
    }
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
    text.0 = format!("$ {current_score}");
    score.current_displayed_score = current_score;
}

#[derive(Component)]
struct FloatingScore(Vec3, f32);

fn float_score(
    score_settings: Res<ScoreSettings>,
    time: Res<Time<Physics>>,
    camera: Single<(&Camera, &GlobalTransform)>,
    mut floatys: Query<(Entity, &mut Node, &mut FloatingScore, &mut TextColor)>,
    mut commands: Commands,
) {
    let (camera, global_transform) = camera.into_inner();
    for (entity, mut node, mut floaty, mut color) in &mut floatys {
        floaty.1 += time.delta_secs();
        color
            .0
            .set_alpha(1.0 - floaty.1 * score_settings.floating_score_fadeout_speed);
        let screen_space = camera
            .world_to_viewport(global_transform, floaty.0)
            .unwrap();
        let top = screen_space.y - floaty.1 * score_settings.floating_score_speed;
        node.top = Val::Px(top);
        node.left = Val::Px(screen_space.x);
        if top < 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

fn on_score_event(
    trigger: Trigger<ScoreEvent>,
    score_settings: Res<ScoreSettings>,
    mut score: ResMut<Score>,
    font_assets: Res<FontAssets>,
    mut next_state: ResMut<NextState<Gameplay>>,
    enemies: Query<&Health, With<Enemy>>,
    mut commands: Commands,
) {
    match trigger.event() {
        ScoreEvent::AddScore(dollars, position) => {
            score.current_t = 0.0;
            score.actual_score += dollars;
            score.old_score = score.current_displayed_score;

            let font_size = score_settings.min_font_size.lerp(
                score_settings.max_font_size,
                *dollars / score_settings.max_font_size_score,
            );
            let color = Color::hsv(0.0, *dollars / score_settings.max_font_size_score, 1.0);
            commands.spawn((
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::Flex,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                Text::from(format!("$ {dollars}")),
                TextLayout::new_with_justify(JustifyText::Center),
                TextFont {
                    font: font_assets.content.clone(),
                    font_size,
                    ..default()
                },
                TextColor(color),
                StateScoped(Screen::Gameplay),
                FloatingScore(*position, 0.0),
            ));
        }
        ScoreEvent::EnemyDeath => {
            if enemies.is_empty() {
                commands.insert_resource(Winner::Player);
                next_state.set(Gameplay::GameOver);
            }
        }
        ScoreEvent::PlayerDeath => {
            commands.insert_resource(Winner::Enemy);
            next_state.set(Gameplay::GameOver);
        }
    };
}

#[derive(Event)]
pub enum ScoreEvent {
    AddScore(f32, Vec3),
    EnemyDeath,
    PlayerDeath,
}

#[derive(PartialEq, Reflect, Resource, Default)]
#[reflect(Resource)]
pub enum Winner {
    Player,
    #[default]
    Enemy,
}

#[derive(Default, Reflect, Resource)]
#[reflect(Resource)]
pub struct Score {
    actual_score: f32,
    current_displayed_score: f32,
    old_score: f32,
    current_t: f32,
}

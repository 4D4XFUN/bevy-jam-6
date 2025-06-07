use avian3d::prelude::Physics;
use bevy::{
    color::palettes::css::{BLACK, WHITE},
    prelude::*,
};

use crate::theme::film_grain::FilmGrainSettingsTween;
use crate::{
    gameplay::{Gameplay, enemy::Enemy, health_and_damage::Health},
    screens::Screen,
    theme::widget,
    ui_assets::{FontAssets, PanelAssets},
};

pub fn plugin(app: &mut App) {
    app.register_type::<Score>()
        .add_systems(
            OnEnter(Gameplay::GameOver),
            (
                setup,
                FilmGrainSettingsTween::tween_close_vignette_to_black_screen,
            ),
        )
        .add_systems(OnEnter(Screen::Retry), retry)
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
        .add_observer(on_score_event);
}

fn setup(
    panel: Res<PanelAssets>,
    score: Res<Score>,
    winner: Res<Winner>,
    font_assets: Res<FontAssets>,
    mut commands: Commands,
) {
    let text = match *winner {
        Winner::Player => format!("You claimed $ {:05} as bounty", score.actual_score),
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
                Text::new("GAME OVER"),
                TextFont::from_font_size(40.0).with_font(font_assets.header.clone()),
            ));
            parent.spawn((
                Name::new("Label"),
                Text(text),
                TextFont::from_font_size(24.0).with_font(font_assets.content.clone()),
            ));
            if Winner::Player == *winner {
                parent.spawn(widget::paneled_button(
                    "Next Level",
                    retry_level,
                    &panel,
                    &font_assets.header,
                ));
            }
            parent.spawn(widget::paneled_button(
                "Retry",
                retry_level,
                &panel,
                &font_assets.header,
            ));
            parent.spawn(widget::paneled_button(
                "Main Menu",
                main_menu,
                &panel,
                &font_assets.header,
            ));
        });
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
    text.0 = format!("$ {current_score:05}");
    score.current_displayed_score = current_score;
}

fn on_score_event(
    trigger: Trigger<ScoreEvent>,
    mut score: ResMut<Score>,
    mut next_state: ResMut<NextState<Gameplay>>,
    enemies: Query<&Health, With<Enemy>>,
    mut commands: Commands,
) {
    match trigger.event() {
        ScoreEvent::AddScore(dollars) => {
            score.current_t = 0.0;
            score.actual_score += dollars;
            score.old_score = score.current_displayed_score;
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
    AddScore(f32),
    EnemyDeath,
    PlayerDeath,
}

#[derive(PartialEq, Reflect, Resource)]
#[reflect(Resource)]
pub enum Winner {
    Player,
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

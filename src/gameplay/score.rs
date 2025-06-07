use bevy::prelude::*;

use crate::{
    gameplay::Gameplay, screens::Screen, theme::widget, ui_assets::{FontAssets, PanelAssets}
};

pub fn plugin(app: &mut App) {
    app.add_systems(OnEnter(Gameplay::GameOver), setup)
    .add_systems(OnEnter(Screen::Retry), retry);
}

fn setup(panel: Res<PanelAssets>, font_assets: Res<FontAssets>, mut commands: Commands) {
    commands.spawn((
        widget::ui_root("Title Screen"),
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

fn retry_level(_trigger: Trigger<Pointer<Click>>, mut next_state: ResMut<NextState<Screen>>) {
    next_state.set(Screen::Retry);
}

fn main_menu(_trigger: Trigger<Pointer<Click>>, mut next_state: ResMut<NextState<Screen>>) {
    next_state.set(Screen::Title);
}

fn retry(mut next_state: ResMut<NextState<Screen>>) {
    next_state.set(Screen::Gameplay);
}

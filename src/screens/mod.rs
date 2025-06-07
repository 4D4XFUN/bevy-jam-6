//! The game's main screen states and transitions between them.

mod credits;
mod gameplay;
mod loading;
pub mod settings;
mod splash;
mod title;

use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.init_state::<Screen>();

    app.add_plugins((
        credits::plugin,
        gameplay::plugin,
        loading::plugin,
        settings::plugin,
        splash::plugin,
        title::plugin,
    ));
}

/// The game's main screen states.
#[derive(States, Debug, Hash, PartialEq, Eq, Clone, Default)]
#[states(scoped_entities)]
pub enum Screen {
    Splash,
    Title,
    Credits,
    Settings,
    #[default]
    Loading,
    Gameplay,
    /// This state exists to make retrying a level easier
    Retry,
}

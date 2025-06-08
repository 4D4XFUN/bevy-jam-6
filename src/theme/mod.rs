//! Reusable UI widgets & theming.

// Unused utilities may trigger this lints undesirably.
#![allow(dead_code)]

pub mod film_grain;
pub mod interaction;
pub mod palette;
pub mod widget;
pub mod particles;

#[allow(unused_imports)]
pub mod prelude {
    pub use super::{interaction::InteractionPalette, palette as ui_palette, widget};
}

use crate::theme::film_grain::{FilmGrainPlugin, update_film_grain_time};
use bevy::prelude::*;

pub(super) fn plugin(app: &mut App) {
    app.add_plugins(interaction::plugin);

    // grain
    app.add_plugins(FilmGrainPlugin);
    app.add_systems(Update, update_film_grain_time);
    
    // particles
    app.add_plugins(particles::plugin);
}

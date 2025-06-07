use avian3d::prelude::{Physics, PhysicsTime};
use bevy::asset::Handle;
use bevy::audio::{
    AudioPlayer, AudioSink, AudioSinkPlayback, AudioSource, PlaybackSettings, Volume,
};
use bevy::ecs::system::{Query, Res};
use bevy::prelude::{Bundle, Component};
use bevy::time::Time;

/// An organizational marker component that should be added to a spawned [`AudioPlayer`] if it's in the
/// general "music" category (e.g. global background music, soundtrack).
///
/// This can then be used to query for and operate on sounds in that category.
#[derive(Component, Default)]
pub struct Music;

/// A music audio instance.
pub fn music(handle: Handle<AudioSource>) -> impl Bundle {
    (AudioPlayer(handle), PlaybackSettings::LOOP, Music)
}

/// An organizational marker component that should be added to a spawned [`AudioPlayer`] if it's in the
/// general "sound effect" category (e.g. footsteps, the sound of a magic spell, a door opening).
///
/// This can then be used to query for and operate on sounds in that category.
#[derive(Component, Default)]
pub struct SoundEffect;

/// A sound effect audio instance.
pub fn sound_effect(handle: Handle<AudioSource>) -> impl Bundle {
    (
        AudioPlayer(handle),
        PlaybackSettings::DESPAWN,
        TimeDilatedPitch(1.0),
    )
}

pub fn sound_effect_non_dilated(handle: Handle<AudioSource>, decibels: f32) -> impl Bundle {
    (
        AudioPlayer(handle),
        PlaybackSettings::DESPAWN.with_volume(Volume::Decibels(decibels)),
    )
}

#[derive(Component)]
pub struct TimeDilatedPitch(pub f32);

pub fn update_sfx_speed(time: Res<Time<Physics>>, query: Query<(&AudioSink, &TimeDilatedPitch)>) {
    for (sink, sfx) in &query {
        sink.set_speed(time.relative_speed() * sfx.0);
    }
}

#![warn(missing_docs)]

//! A plugin which adds MIDI file and soundfont audio support to the [bevy](https://crates.io/crates/bevy) engine via [rustysynth](https://crates.io/crates/rustysynth).

use bevy::{audio::AddAudioSource, prelude::*};
use rustysynth::SoundFont;
use std::{
    io::{Cursor, Read},
    sync::{Arc, OnceLock},
};

mod assets;
pub use assets::*;

#[cfg(feature = "hl4mgm")]
pub(crate) static HL4MGM: &[u8] = include_bytes!("./embedded_assets/hl4mgm.sf2");

pub(crate) static SOUNDFONT: OnceLock<Arc<SoundFont>> = OnceLock::new();

/// This plugin configures the soundfont used for playback and registers MIDI assets.
#[derive(Debug)]
pub struct RustySynthPlugin<R: Read + Send + Sync + Clone + 'static> {
    /// Reader for soundfont data.
    pub soundfont: R,
}

#[cfg(feature = "hl4mgm")]
impl Default for RustySynthPlugin<Cursor<&[u8]>> {
    fn default() -> Self {
        Self {
            soundfont: Cursor::new(HL4MGM),
        }
    }
}

impl<R: Read + Send + Sync + Clone + 'static> Plugin for RustySynthPlugin<R> {
    fn build(&self, app: &mut App) {
        let _ = SOUNDFONT.set(Arc::new(
            SoundFont::new(&mut self.soundfont.clone()).unwrap(),
        ));
        app.add_audio_source::<MidiAudio>()
            .init_asset::<MidiAudio>()
            .init_asset_loader::<MidiAssetLoader>();
    }
}

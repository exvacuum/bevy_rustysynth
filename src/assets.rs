use std::{
    io::{self, Cursor},
    sync::Arc,
    time::Duration,
};

use async_channel::{Receiver, TryRecvError};
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    audio::Source,
    prelude::*,
    tasks::AsyncComputeTaskPool,
};
use itertools::Itertools;
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

/// Represents a single MIDI note in a sequence
#[derive(Clone, Debug)]
pub struct MidiNote {
    /// Channel to play the note on
    pub channel: i32,
    /// Preset (instrument) to play the note with (see GM spec.)
    pub preset: i32,
    /// Key to play (60 is middle C)
    pub key: i32,
    /// Velocity to play note at
    pub velocity: i32,
    /// Duration to play note for
    pub duration: Duration,
}

impl Default for MidiNote {
    fn default() -> Self {
        Self {
            channel: 0,
            preset: 0,
            key: 60,
            velocity: 100,
            duration: Duration::from_secs(1),
        }
    }
}

/// MIDI audio asset
#[derive(Asset, TypePath, Clone, Debug)]
pub enum MidiAudio {
    /// Plays audio from a MIDI file
    File(Vec<u8>),
    /// Plays a simple sequence of notes
    Sequence(Vec<MidiNote>),
}

/// AssetLoader for MIDI files (.mid/.midi)
#[derive(Default, Debug)]
pub struct MidiAssetLoader;

impl AssetLoader for MidiAssetLoader {
    type Asset = MidiAudio;

    type Settings = ();

    type Error = io::Error;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a Self::Settings,
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = vec![];
        reader.read_to_end(&mut bytes).await?;
        Ok(MidiAudio::File(bytes))
    }

    fn extensions(&self) -> &[&str] {
        &["mid", "midi"]
    }
}

/// Decoder for MIDI file playback
pub struct MidiFileDecoder {
    sample_rate: usize,
    stream: Receiver<f32>,
}

impl MidiFileDecoder {
    /// Construct and begin a new MIDI sequencer with the given MIDI data and soundfont.
    ///
    /// The sequencer will push at most 1 second's worth of audio ahead, allowing the decoder to
    /// be paused without endlessly backing up data forever.
    pub fn new(midi: MidiAudio, soundfont: Arc<SoundFont>) -> Self {
        let sample_rate = 44100_usize;
        let (tx, rx) = async_channel::bounded::<f32>(sample_rate * 2);
        AsyncComputeTaskPool::get().spawn(async move {
            let settings = SynthesizerSettings::new(sample_rate as i32);
            let mut synthesizer =
                Synthesizer::new(&soundfont, &settings).expect("Failed to create synthesizer.");

            match midi {
                MidiAudio::File(midi_data) => {
                    let mut sequencer = MidiFileSequencer::new(synthesizer);
                    let mut midi_data = Cursor::new(midi_data);
                    let midi =
                        Arc::new(MidiFile::new(&mut midi_data).expect("Failed to read midi file."));
                    sequencer.play(&midi, false);
                    let mut left: Vec<f32> = vec![0_f32; sample_rate];
                    let mut right: Vec<f32> = vec![0_f32; sample_rate];
                    while !sequencer.end_of_sequence() {
                        sequencer.render(&mut left, &mut right);
                        for value in left.iter().interleave(right.iter()) {
                            if let Err(_) = tx.send(*value).await {
                                return;
                            };
                        }
                    }
                }
                MidiAudio::Sequence(sequence) => {
                    for MidiNote {
                        channel,
                        preset,
                        key,
                        velocity,
                        duration,
                    } in sequence.iter()
                    {
                        synthesizer.process_midi_message(*channel, 0b1100_0000, *preset, 0);
                        synthesizer.note_on(*channel, *key, *velocity);
                        let note_length = (sample_rate as f32 * duration.as_secs_f32()) as usize;
                        let mut left: Vec<f32> = vec![0_f32; note_length];
                        let mut right: Vec<f32> = vec![0_f32; note_length];
                        synthesizer.render(&mut left, &mut right);
                        for value in left.iter().interleave(right.iter()) {
                            if let Err(_) = tx.send(*value).await {
                                return;
                            };
                        }
                        synthesizer.note_off(*channel, *key);
                    }
                }
            }

            tx.close();
        }).detach();
        Self {
            sample_rate,
            stream: rx,
        }
    }
}

impl Iterator for MidiFileDecoder {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        match self.stream.try_recv() {
            Ok(value) => Some(value),
            Err(e) => match e {
                TryRecvError::Empty => Some(0.0),
                TryRecvError::Closed => None,
            },
        }
    }
}

impl Source for MidiFileDecoder {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> u16 {
        2
    }

    fn sample_rate(&self) -> u32 {
        self.sample_rate as u32
    }

    fn total_duration(&self) -> Option<std::time::Duration> {
        None
    }
}

impl Decodable for MidiAudio {
    type Decoder = MidiFileDecoder;

    type DecoderItem = <MidiFileDecoder as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        MidiFileDecoder::new(self.clone(), crate::SOUNDFONT.get().unwrap().clone())
    }
}

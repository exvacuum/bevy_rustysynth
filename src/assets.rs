use std::{
    io::{self, Cursor},
    sync::Arc,
};

use async_channel::{Receiver, TryRecvError};
use bevy::{
    asset::{io::Reader, AssetLoader, AsyncReadExt, LoadContext},
    audio::Source,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use itertools::Itertools;
use rustysynth::{MidiFile, MidiFileSequencer, SoundFont, Synthesizer, SynthesizerSettings};

/// MIDI audio asset
#[derive(Asset, TypePath)]
pub struct MidiAudio {
    /// MIDI file data
    pub midi: Vec<u8>,
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
        Ok(MidiAudio { midi: bytes })
    }
}

/// Decoder for MIDI file playback
pub struct MidiDecoder {
    sample_rate: usize,
    stream: Receiver<f32>,
    _task: Task<()>,
}

impl MidiDecoder {
    /// Construct and begin a new MIDI sequencer with the given MIDI data and soundfont.
    ///
    /// The sequencer will push at most 1 second's worth of audio ahead, allowing the decoder to
    /// be paused without endlessly backing up data forever.
    pub fn new(midi: Vec<u8>, soundfont: Arc<SoundFont>) -> Self {
        let mut midi = Cursor::new(midi);
        let sample_rate = 44100_usize;
        let (tx, rx) = async_channel::bounded::<f32>(sample_rate * 2);
        let task = AsyncComputeTaskPool::get()
            .spawn(async move {
                let midi = Arc::new(MidiFile::new(&mut midi).expect("Failed to read midi file."));
                let settings = SynthesizerSettings::new(sample_rate as i32);
                let synthesizer =
                    Synthesizer::new(&soundfont, &settings).expect("Failed to create synthesizer.");
                let mut sequencer = MidiFileSequencer::new(synthesizer);
                sequencer.play(&midi, true);

                let mut left: Vec<f32> = vec![0_f32; sample_rate];
                let mut right: Vec<f32> = vec![0_f32; sample_rate];
                while !sequencer.end_of_sequence() {
                    sequencer.render(&mut left, &mut right);
                    for value in left.iter().interleave(right.iter()) {
                        if let Err(e) = tx.send(*value).await {
                            error!("{e}");
                        };
                    }
                }
                tx.close();
            });
        Self {
            _task: task,
            sample_rate,
            stream: rx,
        }
    }
}

impl Iterator for MidiDecoder {
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

impl Source for MidiDecoder {
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
    type Decoder = MidiDecoder;

    type DecoderItem = <MidiDecoder as Iterator>::Item;

    fn decoder(&self) -> Self::Decoder {
        MidiDecoder::new(self.midi.clone(), crate::SOUNDFONT.get().unwrap().clone())
    }
}

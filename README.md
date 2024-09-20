# bevy_rustysynth

![Crates](https://img.shields.io/crates/v/bevy_rustysynth)
![License](https://img.shields.io/badge/license-0BSD%2FMIT%2FApache-blue.svg)
![Tag](https://img.shields.io/github/v/tag/exvacuum/bevy_rustysynth)
![Build](https://img.shields.io/github/actions/workflow/status/exvacuum/bevy_rustysynth/rust.yml)

A plugin which adds MIDI file and soundfont audio support to the bevy engine via rustysynth.

## Compatibility

| Crate Version | Bevy Version |
|---            |---           |
| 0.1-0.2       | 0.14         |

## Installation

### crates.io
```toml
[dependencies]
bevy_rustysynth = "0.2"
```

### Using git URL in Cargo.toml
```toml
[dependencies.bevy_rustysynth]
git = "https://github.com/exvacuum/bevy_rustysynth.git"
```

## Usage

In `main.rs`:
```rs
use bevy::prelude::*;
use bevy_rustysynth::*;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            RustySynthPlugin {
                soundfont: // Bring your own soundfont or enable the "hl4mgm" feature to use a terrible 4MB default
            }
        ))
        .run();
}
```
Then you can load and play a MIDI like any other audio file:
```rs
let midi_handle = asset_server.load::<MidiAudio>("example.mid");

commands.spawn(AudioSourceBundle {
    source: midi_handle,
    ..Default::default()
});
```

## License

This crate is licensed under your choice of 0BSD, Apache-2.0, or MIT license.


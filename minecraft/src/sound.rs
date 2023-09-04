//! Audio abstractions.

use std::{
    io::Cursor,
    fmt::{self, Formatter, Debug},
};
use rodio::{
    decoder::Decoder,
    source::{
        Source,
        Buffered,
    },
    OutputStream,
    OutputStreamHandle,
};
use rand::Rng;
use rand_pcg::Pcg32;
use spin::Mutex;
use anyhow::Result;


/// A single clip of sound which can be played.
#[derive(Clone)]
pub struct SoundClip(Buffered<Decoder<Cursor<Vec<u8>>>>);

impl SoundClip {
    pub fn new(file_data: Vec<u8>) -> Result<Self> {
        Ok(SoundClip(Decoder::new(Cursor::new(file_data))?.buffered()))
    }
}

impl Debug for SoundClip {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Sound(..)")
    }
}


/// A nonempty set of sound clips to be selected from upon playing.
#[derive(Debug, Clone)]
pub struct SoundEffect(Vec<SoundClip>);

impl SoundEffect {
    /// Panics if variants is empty.
    pub fn new(variants: Vec<SoundClip>) -> Self {
        SoundEffect(variants)
    }
}

impl From<SoundClip> for SoundEffect {
    fn from(clip: SoundClip) -> Self {
        SoundEffect(vec![clip])
    }
}


/// Object for physically playing sounds.
pub struct SoundPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    rng: Mutex<Pcg32>,
}

impl SoundPlayer {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        Ok(SoundPlayer {
            _stream: stream,
            stream_handle,
            rng: Mutex::new(Pcg32::new(
                0xcafef00dd15ea5e5,
                0x0a02bdbf7bb3c0a7,
            )),
        })
    }

    pub fn play(&self, sound: &SoundEffect, volume: f32) {
        let r = self.rng.lock().gen::<usize>();
        let clip = &sound.0[r % sound.0.len()];

        let source = clip.0.clone().convert_samples();
        let source = source.amplify(volume);
        let res = self.stream_handle.play_raw(source);
        if let Err(e) = res {
            error!(%e, "error playing sound");
        }
    }
}

impl Debug for SoundPlayer {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("SoundPlayer(..)")
    }
}

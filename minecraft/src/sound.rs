//! Audio abstractions.

use std::{
    path::Path,
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
use tokio::fs;
use anyhow::*;


/// A clip of audio, or a set of audio clips for one to be randomly selected.
#[derive(Clone)]
pub struct SoundEffect(Buffered<Decoder<Cursor<Vec<u8>>>>);

impl SoundEffect {
    pub fn load(file_data: Vec<u8>) -> Result<Self> {
        Ok(Sound(Decoder::new(Cursor::new(file_data))?.buffered()))
    }

    pub async fn read_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::new(fs::read(path).await?)
    }
}

impl Debug for Sound {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.write_str("Sound(..)")
    }
}





pub struct SoundPlayer {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
}

impl SoundPlayer {
    pub fn new() -> Result<Self> {
        let (stream, stream_handle) = OutputStream::try_default()?;
        Ok(SoundPlayer {
            _stream: stream,
            stream_handle,
        })
    }

    pub fn play(&self, sound: &Sound) {
        let res = self.stream_handle.play_raw(sound.0.clone().convert_samples());
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

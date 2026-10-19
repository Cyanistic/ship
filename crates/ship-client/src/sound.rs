//! Notification sounds: clips loaded with the config, played on a thread
//! that opens the audio device for each one.

use std::{
    fs,
    io::Cursor,
    path::Path,
    sync::{Arc, mpsc},
    thread,
};

use rodio::{Decoder, DeviceSinkBuilder, Player};
use ship_core::prelude::*;

use crate::keymap::Notice;

/// Audio bytes whose header decodes; cheap to clone.
#[derive(Clone)]
pub struct Clip(Arc<[u8]>);

impl Clip {
    /// The notice's bundled sound.
    pub fn bundled(notice: Notice) -> Self {
        let bytes: &[u8] = match notice {
            Notice::Done => include_bytes!("../assets/sounds/done.wav"),
            Notice::Attention => include_bytes!("../assets/sounds/attention.wav"),
        };
        Self(bytes.into())
    }

    /// Reads `path` and probes it with rodio's decoder.
    pub fn load(path: &Path) -> Result<Self> {
        let bytes: Arc<[u8]> = fs::read(path)
            .map_err(|error| err!(Io, "cannot read sound {}", path.display(), @external: error))?
            .into();
        Decoder::new(Cursor::new(Arc::clone(&bytes))).map_err(
            |error| err!(Configuration, "cannot decode sound {}", path.display(), @external: error),
        )?;
        Ok(Self(bytes))
    }
}

/// A loaded slot: silence or a clip.
#[derive(Clone)]
pub enum Cue {
    Off,
    Play(Clip),
}

/// Dropping it ends the thread.
pub struct Speaker(mpsc::Sender<Clip>);

impl Speaker {
    /// Starts the thread. It opens the default output for each clip and
    /// closes it once the clip ends, since an open output keeps macOS from
    /// idle sleep. The first failure to open is logged; later clips try
    /// again quietly.
    pub fn start() -> Self {
        let (sender, clips) = mpsc::channel::<Clip>();
        let spawned = thread::Builder::new()
            .name("speaker".into())
            .spawn(move || {
                let mut warned = false;
                for clip in clips {
                    let source = match Decoder::new(Cursor::new(clip.0)) {
                        Ok(source) => source,
                        Err(error) => {
                            tracing::warn!("cannot decode a notification sound: {error}");
                            continue;
                        }
                    };
                    let mut sink = match DeviceSinkBuilder::open_default_sink() {
                        Ok(sink) => sink,
                        Err(error) => {
                            if !warned {
                                tracing::warn!("cannot open audio output for sounds: {error}");
                                warned = true;
                            }
                            continue;
                        }
                    };
                    sink.log_on_drop(false);
                    let player = Player::connect_new(sink.mixer());
                    player.append(source);
                    player.sleep_until_end();
                }
            });
        if let Err(error) = spawned {
            tracing::warn!("no notification sounds: cannot start the audio thread: {error}");
        }
        Self(sender)
    }

    pub fn play(&self, clip: Clip) {
        // A send fails only when the thread couldn't start.
        self.0.send(clip).ok();
    }
}

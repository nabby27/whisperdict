use crate::audio::{AudioBuffer, Recorder};
use anyhow::{Context, Result};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

enum Command {
    Start,
    Stop(Sender<AudioBuffer>),
}

#[derive(Clone)]
pub struct RecorderWorker {
    tx: Sender<Command>,
    recording: Arc<AtomicBool>,
}

impl RecorderWorker {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<Command>();
        let recording = Arc::new(AtomicBool::new(false));
        let recording_flag = recording.clone();

        thread::spawn(move || {
            let mut recorder: Option<Recorder> = None;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    Command::Start => {
                        if recorder.is_none() {
                            if let Ok(r) = Recorder::start() {
                                recorder = Some(r);
                                recording_flag.store(true, Ordering::SeqCst);
                            }
                        }
                    }
                    Command::Stop(reply) => {
                        if let Some(active) = recorder.take() {
                            recording_flag.store(false, Ordering::SeqCst);
                            if let Ok(buffer) = active.stop() {
                                let _ = reply.send(buffer);
                            }
                        } else {
                            let _ = reply.send(AudioBuffer {
                                samples: Vec::new(),
                                sample_rate: 16_000,
                            });
                        }
                    }
                }
            }
        });

        Self { tx, recording }
    }

    pub fn start(&self) -> Result<()> {
        self.tx.send(Command::Start).context("start recording")?;
        Ok(())
    }

    pub fn stop(&self) -> Result<AudioBuffer> {
        let (tx, rx) = mpsc::channel();
        self.tx.send(Command::Stop(tx)).context("stop recording")?;
        let buffer = rx.recv().context("receive audio")?;
        Ok(buffer)
    }

    pub fn is_recording(&self) -> bool {
        self.recording.load(Ordering::SeqCst)
    }
}

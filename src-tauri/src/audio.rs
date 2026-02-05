use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, Sample, SampleFormat, SizedSample, Stream};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct AudioBuffer {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

pub struct Recorder {
    stream: Stream,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
}

impl Recorder {
    pub fn start() -> Result<Self> {
        let host = cpal::default_host();
        let device = host.default_input_device().context("no input device")?;
        let supported = device
            .supported_input_configs()
            .context("no input configs")?;

        let mut chosen_config = None;
        for config in supported {
            let config = config.with_max_sample_rate();
            if config.channels() == 1 && config.sample_rate().0 == 16_000 {
                chosen_config = Some(config);
                break;
            }
        }

        let default_config = device
            .default_input_config()
            .context("default input config")?;
        let chosen = chosen_config.unwrap_or(default_config);
        let sample_format = chosen.sample_format();
        let config = chosen.config();

        let sample_rate = config.sample_rate.0;
        let channels = config.channels;
        let samples = Arc::new(Mutex::new(Vec::new()));

        let samples_ref = samples.clone();
        let err_fn = move |err| {
            eprintln!("audio stream error: {err}");
        };

        let stream = match sample_format {
            SampleFormat::F32 => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    push_samples(data, channels, &samples_ref);
                },
                err_fn,
                None,
            )?,
            SampleFormat::I16 => device.build_input_stream(
                &config,
                move |data: &[i16], _| {
                    push_samples(data, channels, &samples_ref);
                },
                err_fn,
                None,
            )?,
            SampleFormat::U16 => device.build_input_stream(
                &config,
                move |data: &[u16], _| {
                    push_samples(data, channels, &samples_ref);
                },
                err_fn,
                None,
            )?,
            _ => device.build_input_stream(
                &config,
                move |data: &[f32], _| {
                    push_samples(data, channels, &samples_ref);
                },
                err_fn,
                None,
            )?,
        };

        stream.play()?;

        Ok(Self {
            stream,
            samples,
            sample_rate,
        })
    }

    pub fn stop(self) -> Result<AudioBuffer> {
        drop(self.stream);
        let samples = self.samples.lock().unwrap().clone();
        Ok(AudioBuffer {
            samples,
            sample_rate: self.sample_rate,
        })
    }
}

fn push_samples<T: Sample + SizedSample>(data: &[T], channels: u16, buffer: &Arc<Mutex<Vec<f32>>>)
where
    f32: FromSample<T>,
{
    let mut guard = buffer.lock().unwrap();
    if channels == 1 {
        guard.extend(data.iter().map(|s| s.to_sample::<f32>()));
        return;
    }

    let mut idx = 0;
    while idx + channels as usize <= data.len() {
        let mut sum = 0.0f32;
        for channel in 0..channels as usize {
            sum += data[idx + channel].to_sample::<f32>();
        }
        guard.push(sum / channels as f32);
        idx += channels as usize;
    }
}

pub fn resample_to_16k(buffer: AudioBuffer) -> AudioBuffer {
    if buffer.sample_rate == 16_000 {
        return buffer;
    }

    let ratio = 16_000.0 / buffer.sample_rate as f32;
    let out_len = (buffer.samples.len() as f32 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f32 / ratio;
        let idx = src_pos.floor() as usize;
        let frac = src_pos - idx as f32;
        let a = buffer.samples.get(idx).copied().unwrap_or(0.0);
        let b = buffer.samples.get(idx + 1).copied().unwrap_or(a);
        out.push(a + (b - a) * frac);
    }

    AudioBuffer {
        samples: out,
        sample_rate: 16_000,
    }
}

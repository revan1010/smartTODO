use std::sync::Arc;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use parking_lot::Mutex;

const TARGET_SAMPLE_RATE: u32 = 16_000;

pub struct Recorder {
    stream: Option<cpal::Stream>,
    samples: Arc<Mutex<Vec<f32>>>,
    source_sample_rate: u32,
    source_channels: u16,
}

unsafe impl Send for Recorder {}

impl Recorder {
    pub fn start() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "no default input device".to_string())?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("default_input_config: {e}"))?;

        let source_sample_rate = config.sample_rate().0;
        let source_channels = config.channels();
        let samples: Arc<Mutex<Vec<f32>>> = Arc::new(Mutex::new(Vec::new()));
        let samples_cb = samples.clone();

        let err_fn = |err| eprintln!("cpal stream error: {err}");

        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_input_stream(
                &config.into(),
                move |data: &[f32], _| {
                    samples_cb.lock().extend_from_slice(data);
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::I16 => device.build_input_stream(
                &config.into(),
                move |data: &[i16], _| {
                    let mut buf = samples_cb.lock();
                    buf.extend(data.iter().map(|s| *s as f32 / i16::MAX as f32));
                },
                err_fn,
                None,
            ),
            cpal::SampleFormat::U16 => device.build_input_stream(
                &config.into(),
                move |data: &[u16], _| {
                    let mut buf = samples_cb.lock();
                    buf.extend(
                        data.iter()
                            .map(|s| (*s as f32 - u16::MAX as f32 / 2.0) / (u16::MAX as f32 / 2.0)),
                    );
                },
                err_fn,
                None,
            ),
            fmt => return Err(format!("unsupported sample format: {fmt:?}")),
        }
        .map_err(|e| format!("build_input_stream: {e}"))?;

        stream
            .play()
            .map_err(|e| format!("stream.play: {e}"))?;

        Ok(Self {
            stream: Some(stream),
            samples,
            source_sample_rate,
            source_channels,
        })
    }

    pub fn stop_and_take(mut self) -> Vec<f32> {
        if let Some(s) = self.stream.take() {
            let _ = s.pause();
            drop(s);
        }
        let raw = std::mem::take(&mut *self.samples.lock());
        downmix_and_resample(&raw, self.source_channels, self.source_sample_rate)
    }
}

fn downmix_and_resample(samples: &[f32], channels: u16, sample_rate: u32) -> Vec<f32> {
    let channels = channels.max(1) as usize;
    let mono: Vec<f32> = if channels == 1 {
        samples.to_vec()
    } else {
        samples
            .chunks(channels)
            .map(|c| c.iter().sum::<f32>() / channels as f32)
            .collect()
    };

    if sample_rate == TARGET_SAMPLE_RATE {
        return mono;
    }

    let ratio = TARGET_SAMPLE_RATE as f32 / sample_rate as f32;
    let out_len = (mono.len() as f32 * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f32 / ratio;
        let i0 = src.floor() as usize;
        let i1 = (i0 + 1).min(mono.len().saturating_sub(1));
        let frac = src - i0 as f32;
        let s = mono.get(i0).copied().unwrap_or(0.0) * (1.0 - frac)
            + mono.get(i1).copied().unwrap_or(0.0) * frac;
        out.push(s);
    }
    out
}

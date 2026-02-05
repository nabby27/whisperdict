use crate::transcription::transcribe_with_context;
use anyhow::{Context, Result};
use std::env;
use std::io::{self, BufRead, Write};

pub fn run_if_child() -> Result<bool> {
    let mut args = env::args().skip(1);
    let mut is_child = false;
    let mut is_server = false;
    let mut model_path = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--transcribe-child" => is_child = true,
            "--transcribe-server" => {
                is_child = true;
                is_server = true;
            }
            "--model" => model_path = args.next(),
            _ => {}
        }
    }

    if !is_child {
        return Ok(false);
    }

    let model_path = model_path.context("missing model path")?;
    if is_server {
        run_server(&model_path)?;
        return Ok(true);
    }

    Ok(true)
}

fn run_server(model_path: &str) -> Result<()> {
    let mut ctx_params = whisper_rs::WhisperContextParameters::default();
    ctx_params.use_gpu(true);
    let ctx = match whisper_rs::WhisperContext::new_with_params(model_path, ctx_params) {
        Ok(ctx) => ctx,
        Err(err) => {
            eprintln!("ECO-child: GPU init failed ({err}), falling back to CPU");
            let mut cpu_params = whisper_rs::WhisperContextParameters::default();
            cpu_params.use_gpu(false);
            whisper_rs::WhisperContext::new_with_params(model_path, cpu_params)
                .context("load model (cpu)")?
        }
    };
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let wav_path = line.context("read line")?;
        if wav_path.trim().is_empty() {
            continue;
        }
        let text = match transcribe_wav_with_ctx(&ctx, &wav_path) {
            Ok(text) => text,
            Err(err) => {
                eprintln!("ECO-child: error {err}");
                String::new()
            }
        };
        writeln!(stdout, "{}", text).context("write stdout")?;
        stdout.flush().context("flush stdout")?;
    }
    Ok(())
}

fn transcribe_wav_with_ctx(ctx: &whisper_rs::WhisperContext, wav_path: &str) -> Result<String> {
    let reader = hound::WavReader::open(wav_path).context("open wav")?;
    let spec = reader.spec();
    if spec.channels != 1 || spec.sample_rate != 16000 {
        return Err(anyhow::anyhow!("unexpected wav format"));
    }

    let mut samples = Vec::new();
    for sample in reader.into_samples::<i16>() {
        let sample = sample.context("read sample")? as f32 / 32768.0;
        samples.push(sample);
    }

    let text = transcribe_with_context(ctx, &samples, Some("es"), false).context("transcribe")?;
    Ok(text)
}

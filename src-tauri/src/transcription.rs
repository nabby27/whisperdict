use anyhow::{Context, Result};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext};

pub fn transcribe_with_context(
    ctx: &WhisperContext,
    audio: &[f32],
    language: Option<&str>,
    detect_language: bool,
) -> Result<String> {
    if audio.len() < 16_000 / 4 {
        return Ok(String::new());
    }

    let mut cleaned: Vec<f32> = Vec::with_capacity(audio.len());
    let mut max_abs = 0.0f32;
    for &sample in audio {
        let value = if sample.is_finite() { sample } else { 0.0 };
        let value = value.clamp(-1.0, 1.0);
        max_abs = max_abs.max(value.abs());
        cleaned.push(value);
    }

    if max_abs > 1.0 {
        for sample in cleaned.iter_mut() {
            *sample /= max_abs;
        }
    }

    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(4);
    params.set_n_threads(threads.max(2));
    params.set_speed_up(false);
    let lang = if detect_language {
        detect_language_by_scoring(ctx, &cleaned)
            .or(language)
            .or(Some("es"))
    } else {
        language.or(Some("es"))
    };
    params.set_language(lang);
    params.set_detect_language(false);
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);

    let mut state = ctx.create_state().context("create whisper state")?;
    state.full(params, &cleaned).context("transcribe audio")?;

    let segments = state.full_n_segments().context("get segments")?;
    let mut text = String::new();
    for i in 0..segments {
        let segment = state.full_get_segment_text(i).context("segment text")?;
        text.push_str(&segment);
    }
    Ok(text.trim().to_string())
}

fn detect_language_by_scoring(ctx: &WhisperContext, audio: &[f32]) -> Option<&'static str> {
    let sample_len = (16_000.0 * 2.0) as usize;
    let sample = if audio.len() > sample_len {
        &audio[..sample_len]
    } else {
        audio
    };

    let candidates = ["es", "en", "pt", "fr", "de", "it"];
    let mut best_lang = None;
    let mut best_score = f32::MIN;

    for lang in candidates {
        if let Ok(score) = score_language(ctx, sample, lang) {
            if score > best_score {
                best_score = score;
                best_lang = Some(lang);
            }
        }
    }
    best_lang
}

fn score_language(ctx: &WhisperContext, audio: &[f32], lang: &str) -> Result<f32> {
    let mut state = ctx.create_state().context("create whisper state")?;
    let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
    let threads = std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(2);
    params.set_n_threads(threads.max(1));
    params.set_speed_up(false);
    params.set_language(Some(lang));
    params.set_detect_language(false);
    params.set_translate(false);
    params.set_print_progress(false);
    params.set_print_special(false);
    params.set_print_realtime(false);
    params.set_single_segment(true);
    params.set_max_tokens(32);

    state.full(params, audio).context("score transcribe")?;

    let segments = state.full_n_segments().context("score segments")?;
    if segments == 0 {
        return Ok(f32::MIN);
    }
    let mut total_prob = 0.0f32;
    let mut total_tokens = 0i32;
    for segment in 0..segments {
        let tokens = state.full_n_tokens(segment).context("score tokens")?;
        for token in 0..tokens {
            let prob = state.full_get_token_prob(segment, token).unwrap_or(0.0);
            total_prob += prob;
            total_tokens += 1;
        }
    }
    if total_tokens == 0 {
        return Ok(f32::MIN);
    }
    Ok(total_prob / total_tokens as f32)
}

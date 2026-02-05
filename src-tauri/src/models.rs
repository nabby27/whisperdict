use anyhow::{Context, Result};
use directories::BaseDirs;
use futures_util::StreamExt;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone, Serialize)]
pub struct ModelStatus {
    pub id: String,
    pub size_mb: u32,
    pub installed: bool,
    pub partial: bool,
}

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: &'static str,
    pub size_mb: u32,
    pub filename: &'static str,
    pub url: &'static str,
    pub min_bytes: u64,
}

const MODEL_LIST: &[ModelInfo] = &[
    ModelInfo {
        id: "tiny",
        size_mb: 75,
        filename: "ggml-tiny.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.bin",
        min_bytes: 70 * 1024 * 1024,
    },
    ModelInfo {
        id: "base",
        size_mb: 142,
        filename: "ggml-base.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.bin",
        min_bytes: 135 * 1024 * 1024,
    },
    ModelInfo {
        id: "small",
        size_mb: 466,
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        min_bytes: 440 * 1024 * 1024,
    },
    ModelInfo {
        id: "medium",
        size_mb: 1460,
        filename: "ggml-medium.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        min_bytes: 1400 * 1024 * 1024,
    },
    ModelInfo {
        id: "large",
        size_mb: 2880,
        filename: "ggml-large.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large.bin",
        min_bytes: 2700 * 1024 * 1024,
    },
];

pub fn models_dir() -> Result<PathBuf> {
    let dirs = BaseDirs::new().context("missing base dirs")?;
    let dir = dirs.data_local_dir().join("eco").join("models");
    fs::create_dir_all(&dir).context("create models dir")?;
    Ok(dir)
}

pub fn list_models() -> Result<Vec<ModelStatus>> {
    let dir = models_dir()?;
    let items = MODEL_LIST
        .iter()
        .map(|model| ModelStatus {
            id: model.id.to_string(),
            size_mb: model.size_mb,
            installed: dir.join(model.filename).exists() && model_is_valid(model.id).unwrap_or(false),
            partial: dir.join(format!("{}.part", model.filename)).exists(),
        })
        .collect();
    Ok(items)
}

pub fn get_model_info(model_id: &str) -> Option<&'static ModelInfo> {
    MODEL_LIST.iter().find(|model| model.id == model_id)
}

pub fn model_path(model_id: &str) -> Result<PathBuf> {
    let dir = models_dir()?;
    let info = get_model_info(model_id).context("unknown model")?;
    Ok(dir.join(info.filename))
}

pub fn model_is_valid(model_id: &str) -> Result<bool> {
    let info = get_model_info(model_id).context("unknown model")?;
    let path = model_path(model_id)?;
    if !path.exists() {
        return Ok(false);
    }
    let metadata = fs::metadata(path).context("model metadata")?;
    Ok(metadata.len() >= info.min_bytes)
}

pub fn delete_model(model_id: &str) -> Result<()> {
    let info = get_model_info(model_id).context("unknown model")?;
    let dir = models_dir()?;
    let path = dir.join(info.filename);
    let part = dir.join(format!("{}.part", info.filename));
    if path.exists() {
        let _ = fs::remove_file(&path);
    }
    if part.exists() {
        let _ = fs::remove_file(&part);
    }
    Ok(())
}

pub async fn download_model_with_progress<F>(model_id: &str, progress: F) -> Result<PathBuf>
where
    F: Fn(u64, Option<u64>) + Send + Sync,
{
    let info = get_model_info(model_id).context("unknown model")?;
    let dir = models_dir()?;
    let path = dir.join(info.filename);
    let temp_path = dir.join(format!("{}.part", info.filename));
    if temp_path.exists() {
        let _ = tokio::fs::remove_file(&temp_path).await;
    }
    if path.exists() {
        if !model_is_valid(model_id)? {
            let _ = tokio::fs::remove_file(&path).await;
        } else {
            return Ok(path);
        }
    }

    let mut file = tokio::fs::File::create(&temp_path).await.context("create temp")?;
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60 * 60))
        .build()
        .context("build client")?;
    let response = client
        .get(info.url)
        .send()
        .await
        .context("download model")?
        .error_for_status()
        .context("bad status")?;
    let total = response.content_length();
    let mut downloaded = 0u64;
    let mut stream = response.bytes_stream();

    loop {
        let next = timeout(Duration::from_secs(30), stream.next()).await;
        let item = match next {
            Ok(item) => item,
            Err(_) => {
                let _ = tokio::fs::remove_file(&temp_path).await;
                anyhow::bail!("download stalled for {model_id}");
            }
        };
        let Some(chunk) = item else {
            break;
        };
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(err) => {
                let _ = tokio::fs::remove_file(&temp_path).await;
                return Err(err.into());
            }
        };
        downloaded += chunk.len() as u64;
        file.write_all(&chunk).await.context("write chunk")?;
        progress(downloaded, total);
    }

    file.flush().await.context("flush temp")?;
    tokio::fs::rename(&temp_path, &path).await.context("rename model")?;
    Ok(path)
}

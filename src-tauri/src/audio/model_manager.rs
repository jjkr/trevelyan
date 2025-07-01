use crate::audio::types::{TranscriptionError, TranscriptionResult};
use anyhow::Context;
use candle_core::{Device, Tensor};
use candle_transformers::models::whisper::{self as m, Config};
use hf_hub::api::tokio::Api;
use std::path::{Path, PathBuf};
use tokenizers::Tokenizer;
use tracing::{info, warn};

/// Available Distil-Whisper model variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelVariant {
    DistilTiny,
    DistilSmall,
    DistilMedium,
}

impl ModelVariant {
    pub fn repo_id(&self) -> &'static str {
        match self {
            ModelVariant::DistilTiny => "distil-whisper/distil-small.en",
            ModelVariant::DistilSmall => "distil-whisper/distil-small.en", 
            ModelVariant::DistilMedium => "distil-whisper/distil-medium.en",
        }
    }

    pub fn model_filename(&self) -> &'static str {
        "model.safetensors"
    }

    pub fn config_filename(&self) -> &'static str {
        "config.json"
    }

    pub fn tokenizer_filename(&self) -> &'static str {
        "tokenizer.json"
    }

    pub fn size_mb(&self) -> u64 {
        match self {
            ModelVariant::DistilTiny => 39,
            ModelVariant::DistilSmall => 166,
            ModelVariant::DistilMedium => 394,
        }
    }
}

/// Manages model downloading, caching, and loading
pub struct ModelManager {
    cache_dir: PathBuf,
    device: Device,
}

impl ModelManager {
    pub fn new(cache_dir: PathBuf) -> TranscriptionResult<Self> {
        // Try to use Metal on macOS, fallback to CPU
        let device = if cfg!(target_os = "macos") {
            Device::new_metal(0).unwrap_or_else(|_| {
                warn!("Failed to initialize Metal device, falling back to CPU");
                Device::Cpu
            })
        } else {
            Device::Cpu
        };

        info!("Initialized model manager with device: {:?}", device);

        Ok(Self { cache_dir, device })
    }

    /// Check if a model is already cached locally
    pub fn is_model_cached(&self, variant: ModelVariant) -> bool {
        let model_dir = self.get_model_dir(variant);
        model_dir.join(variant.model_filename()).exists()
            && model_dir.join(variant.config_filename()).exists()
            && model_dir.join(variant.tokenizer_filename()).exists()
    }

    /// Download a model if not already cached
    pub async fn ensure_model_downloaded(&self, variant: ModelVariant) -> TranscriptionResult<()> {
        if self.is_model_cached(variant) {
            info!("Model {:?} already cached", variant);
            return Ok(());
        }

        info!("Downloading model {:?} (~{}MB)", variant, variant.size_mb());
        self.download_model(variant).await?;
        info!("Model {:?} downloaded successfully", variant);
        Ok(())
    }

    /// Download model files from Hugging Face Hub
    async fn download_model(&self, variant: ModelVariant) -> TranscriptionResult<()> {
        let model_dir = self.get_model_dir(variant);
        std::fs::create_dir_all(&model_dir)?;

        let api = Api::new()?;
        let repo = api.model(variant.repo_id().to_string());

        // Download model files
        let files = [
            variant.model_filename(),
            variant.config_filename(),
            variant.tokenizer_filename(),
        ];

        for filename in files {
            let local_path = model_dir.join(filename);
            if !local_path.exists() {
                info!("Downloading {}", filename);
                let remote_path = repo.get(filename).await?;
                std::fs::copy(remote_path, local_path)?;
            }
        }

        Ok(())
    }

    /// Load a model and return the components needed for inference
    pub async fn load_model(&self, variant: ModelVariant) -> TranscriptionResult<LoadedModel> {
        self.ensure_model_downloaded(variant).await?;

        let model_dir = self.get_model_dir(variant);
        
        info!("Loading model {:?} from cache", variant);

        // Load configuration
        let config_path = model_dir.join(variant.config_filename());
        let config_content = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_content)
            .context("Failed to parse model config")?;

        // Load tokenizer
        let tokenizer_path = model_dir.join(variant.tokenizer_filename());
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| TranscriptionError::ModelLoadError(anyhow::anyhow!("Tokenizer load error: {}", e)))?;

        // Load model weights
        let model_path = model_dir.join(variant.model_filename());
        let vb = unsafe {
            candle_nn::VarBuilder::from_mmaped_safetensors(&[model_path], m::DTYPE, &self.device)?
        };

        // Create model
        let model = m::model::Whisper::load(&vb, config.clone())
            .context("Failed to load Whisper model")?;

        info!("Model {:?} loaded successfully", variant);

        Ok(LoadedModel {
            model,
            config,
            tokenizer,
            device: self.device.clone(),
            variant,
        })
    }

    /// Get the directory where a specific model variant is cached
    fn get_model_dir(&self, variant: ModelVariant) -> PathBuf {
        self.cache_dir.join("models").join(format!("{:?}", variant).to_lowercase())
    }

    /// Get cache directory
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// Get device being used
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// Clear cached models (useful for cleanup or forcing re-download)
    pub fn clear_cache(&self) -> TranscriptionResult<()> {
        let models_dir = self.cache_dir.join("models");
        if models_dir.exists() {
            std::fs::remove_dir_all(models_dir)?;
        }
        Ok(())
    }

    /// Get total cache size in bytes
    pub fn get_cache_size(&self) -> TranscriptionResult<u64> {
        let models_dir = self.cache_dir.join("models");
        if !models_dir.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        for entry in std::fs::read_dir(models_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                total_size += self.dir_size(&entry.path())?;
            }
        }

        Ok(total_size)
    }

    fn dir_size(&self, path: &Path) -> TranscriptionResult<u64> {
        let mut size = 0u64;
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let metadata = entry.metadata()?;
            if metadata.is_file() {
                size += metadata.len();
            } else if metadata.is_dir() {
                size += self.dir_size(&entry.path())?;
            }
        }
        Ok(size)
    }
}

/// A loaded model ready for inference
pub struct LoadedModel {
    pub model: m::model::Whisper,
    pub config: Config,
    pub tokenizer: Tokenizer,
    pub device: Device,
    pub variant: ModelVariant,
}

impl LoadedModel {
    /// Get the sample rate expected by this model
    pub fn sample_rate(&self) -> u32 {
        // Whisper models expect 16kHz audio
        16000
    }

    /// Get the maximum audio length in samples that can be processed at once
    pub fn max_audio_samples(&self) -> usize {
        // Whisper processes 30-second chunks at 16kHz
        30 * self.sample_rate() as usize
    }

    /// Preprocess audio data for model input
    pub fn preprocess_audio(&self, audio: &[f32]) -> TranscriptionResult<Tensor> {
        // Ensure audio is the right length (pad or truncate)
        let target_length = self.max_audio_samples();
        let mut processed_audio = vec![0.0f32; target_length];
        
        let copy_length = audio.len().min(target_length);
        processed_audio[..copy_length].copy_from_slice(&audio[..copy_length]);

        // Convert to tensor
        let tensor = Tensor::from_vec(processed_audio, (1, target_length), &self.device)
            .context("Failed to create audio tensor")?;

        Ok(tensor)
    }

    /// Get model information for debugging
    pub fn info(&self) -> ModelInfo {
        ModelInfo {
            variant: self.variant,
            sample_rate: self.sample_rate(),
            max_duration_seconds: self.max_audio_samples() as f32 / self.sample_rate() as f32,
            device: format!("{:?}", self.device),
        }
    }
}

/// Information about a loaded model
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub variant: ModelVariant,
    pub sample_rate: u32,
    pub max_duration_seconds: f32,
    pub device: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_model_variant_properties() {
        assert_eq!(ModelVariant::DistilTiny.repo_id(), "distil-whisper/distil-small.en");
        assert_eq!(ModelVariant::DistilTiny.model_filename(), "model.safetensors");
        assert!(ModelVariant::DistilTiny.size_mb() > 0);
    }

    #[tokio::test]
    async fn test_model_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(manager.cache_dir(), temp_dir.path());
    }

    #[test]
    fn test_model_caching_check() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ModelManager::new(temp_dir.path().to_path_buf()).unwrap();
        
        // Should not be cached initially
        assert!(!manager.is_model_cached(ModelVariant::DistilTiny));
    }
}

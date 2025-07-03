use anyhow::Result;
use candle_core::{Device, Tensor, IndexOp};
use candle_nn::VarBuilder;
use candle_transformers::models::whisper::{self as m, audio, Config};
use hf_hub::{api::sync::Api, Repo, RepoType};
use tokenizers::Tokenizer;
use byteorder::ByteOrder;

// Precomputed mel filters from Candle repository
const MEL_FILTERS_80: &[u8] = include_bytes!("../melfilters.bytes");

pub struct WhisperEngine {
    model: m::model::Whisper,
    tokenizer: Tokenizer,
    config: Config,
    device: Device,
    mel_filters: Vec<f32>,
}

impl WhisperEngine {
    pub async fn new() -> Result<Self> {
        println!("🤖 [whisper_engine] Initializing Whisper engine...");
        let device = Device::Cpu; // Use CPU for better compatibility
        println!("🤖 [whisper_engine] Using device: {:?}", device);
        
        // Download model files
        println!("🤖 [whisper_engine] Creating HuggingFace API client...");
        let api = Api::new()?;
        let repo = api.repo(Repo::new(
            "openai/whisper-tiny.en".to_string(),
            RepoType::Model,
        ));
        println!("🤖 [whisper_engine] Repository created for openai/whisper-tiny.en");

        println!("🤖 [whisper_engine] Downloading model files...");
        let config_path = repo.get("config.json")?;
        println!("🤖 [whisper_engine] Downloaded config.json: {:?}", config_path);
        
        let tokenizer_path = repo.get("tokenizer.json")?;
        println!("🤖 [whisper_engine] Downloaded tokenizer.json: {:?}", tokenizer_path);
        
        let model_path = repo.get("model.safetensors")?;
        println!("🤖 [whisper_engine] Downloaded model.safetensors: {:?}", model_path);

        // Load configuration
        println!("🤖 [whisper_engine] Loading configuration...");
        let config_str = std::fs::read_to_string(config_path)?;
        let config: Config = serde_json::from_str(&config_str)?;
        println!("🤖 [whisper_engine] Config loaded: vocab_size={}, n_mels={}", config.vocab_size, config.num_mel_bins);

        // Load tokenizer
        println!("🤖 [whisper_engine] Loading tokenizer...");
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow::anyhow!("Failed to load tokenizer: {}", e))?;
        println!("🤖 [whisper_engine] Tokenizer loaded with vocab size: {}", tokenizer.get_vocab_size(true));

        // Load mel filters
        println!("🤖 [whisper_engine] Loading mel filters...");
        let mut mel_filters = vec![0f32; MEL_FILTERS_80.len() / 4];
        byteorder::LittleEndian::read_f32_into(MEL_FILTERS_80, &mut mel_filters);
        println!("🤖 [whisper_engine] Mel filters loaded: {} values", mel_filters.len());

        // Load model
        println!("🤖 [whisper_engine] Loading Whisper model...");
        let vb = unsafe { 
            VarBuilder::from_mmaped_safetensors(&[model_path], m::DTYPE, &device)?
        };
        let model = m::model::Whisper::load(&vb, config.clone())?;

        println!("✅ [whisper_engine] Whisper model loaded successfully");

        Ok(Self {
            model,
            tokenizer,
            config,
            device,
            mel_filters,
        })
    }

    pub fn transcribe(&mut self, audio_data: &[f32]) -> Result<String> {
        println!("🤖 [whisper_engine] Starting transcription with {} audio samples", audio_data.len());
        
        if audio_data.is_empty() {
            return Ok(String::new());
        }

        // Convert PCM to mel spectrogram
        let mel = audio::pcm_to_mel(&self.config, audio_data, &self.mel_filters);
        let mel_len = mel.len();
        
        println!("🤖 [whisper_engine] Mel spectrogram length: {}, num_mel_bins: {}", mel_len, self.config.num_mel_bins);
        
        if mel_len == 0 {
            return Ok(String::new());
        }

        // Calculate proper dimensions - Whisper expects exactly 3000 time steps for 30 seconds
        let time_steps = mel_len / self.config.num_mel_bins;
        println!("🤖 [whisper_engine] Calculated time steps: {}", time_steps);
        
        if time_steps == 0 {
            println!("🤖 [whisper_engine] No time steps available, skipping transcription");
            return Ok(String::new());
        }

        // Whisper models expect exactly 3000 time steps (30 seconds at 16kHz)
        let expected_time_steps = 3000;
        let mel_tensor = if time_steps > expected_time_steps {
            // Truncate to expected size
            println!("🤖 [whisper_engine] Truncating from {} to {} time steps", time_steps, expected_time_steps);
            let truncated_len = expected_time_steps * self.config.num_mel_bins;
            let truncated_mel = mel[..truncated_len].to_vec();
            Tensor::from_vec(
                truncated_mel,
                (1, self.config.num_mel_bins, expected_time_steps),
                &self.device,
            )?
        } else if time_steps < expected_time_steps {
            // Pad to expected size
            println!("🤖 [whisper_engine] Padding from {} to {} time steps", time_steps, expected_time_steps);
            let mut padded_mel = mel;
            let padding_needed = (expected_time_steps - time_steps) * self.config.num_mel_bins;
            padded_mel.extend(vec![0.0; padding_needed]);
            Tensor::from_vec(
                padded_mel,
                (1, self.config.num_mel_bins, expected_time_steps),
                &self.device,
            )?
        } else {
            // Perfect size
            Tensor::from_vec(
                mel,
                (1, self.config.num_mel_bins, time_steps),
                &self.device,
            )?
        };
        
        println!("🤖 [whisper_engine] Created mel tensor with shape: {:?}", mel_tensor.shape());

        // Run encoder
        let audio_features = self.model.encoder.forward(&mel_tensor, true)?;

        // Prepare decoder tokens
        let sot_token = self.get_token_id(m::SOT_TOKEN)?;
        let transcribe_token = self.get_token_id(m::TRANSCRIBE_TOKEN)?;
        let no_timestamps_token = self.get_token_id(m::NO_TIMESTAMPS_TOKEN)?;
        let eot_token = self.get_token_id(m::EOT_TOKEN)?;

        let mut tokens = vec![sot_token, transcribe_token, no_timestamps_token];
        
        // Generate tokens
        let max_tokens = 50; // Limit for real-time processing
        for _ in 0..max_tokens {
            let tokens_tensor = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;
            let decoder_output = self.model.decoder.forward(&tokens_tensor, &audio_features, false)?;
            
            let (_, seq_len, _) = decoder_output.dims3()?;
            let logits = self.model.decoder.final_linear(
                &decoder_output.i((..1, seq_len - 1..))?
            )?.i(0)?.i(0)?;

            // Get most likely token
            let logits_vec: Vec<f32> = logits.to_vec1()?;
            let next_token = logits_vec
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                .map(|(i, _)| i as u32)
                .unwrap();

            if next_token == eot_token {
                break;
            }

            tokens.push(next_token);
        }

        // Decode tokens to text
        let text = self.tokenizer
            .decode(&tokens, true)
            .map_err(|e| anyhow::anyhow!("Failed to decode tokens: {}", e))?;

        // Clean up the text (remove special tokens)
        let cleaned_text = text
            .replace("<|startoftranscript|>", "")
            .replace("<|transcribe|>", "")
            .replace("<|notimestamps|>", "")
            .replace("<|endoftext|>", "")
            .trim()
            .to_string();

        Ok(cleaned_text)
    }

    fn get_token_id(&self, token: &str) -> Result<u32> {
        self.tokenizer
            .token_to_id(token)
            .ok_or_else(|| anyhow::anyhow!("Token '{}' not found", token))
    }
}

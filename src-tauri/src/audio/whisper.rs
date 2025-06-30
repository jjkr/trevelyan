use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::fs;
use std::collections::VecDeque;

pub struct WhisperService {
    context: Option<WhisperContext>,
    model_path: PathBuf,
    is_initialized: bool,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl WhisperService {
    pub fn new() -> Self {
        Self {
            context: None,
            model_path: PathBuf::new(),
            is_initialized: false,
            audio_buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn initialize(&mut self, app_data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        self.model_path = app_data_dir.join("models").join("ggml-base.en.bin");
        
        // Create models directory if it doesn't exist
        let models_dir = self.model_path.parent().unwrap();
        fs::create_dir_all(models_dir).await?;

        // Download model if it doesn't exist
        if !self.model_path.exists() {
            self.download_model().await?;
        }

        // Initialize Whisper context with optimized CPU processing
        let mut ctx_params = WhisperContextParameters::default();
        
        // Use optimized CPU processing (avoids Metal shader issues)
        ctx_params.use_gpu(true);
        
        println!("Initializing Whisper with optimized CPU processing...");
        self.context = Some(WhisperContext::new_with_params(
            self.model_path.to_str().unwrap(),
            ctx_params,
        )?);

        println!("Whisper context initialized successfully with optimized CPU processing");
        self.is_initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    pub fn transcribe_chunk(&self, audio_data: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        if !self.is_initialized {
            return Err("Whisper service not initialized".into());
        }

        // Skip if audio data is too short (less than 0.5 seconds)
        if audio_data.len() < 8000 {
            println!("Skipping transcription: audio too short ({} samples)", audio_data.len());
            return Ok(String::new());
        }

        // Analyze audio characteristics for debugging
        let max_amplitude = audio_data.iter().map(|&x| x.abs()).fold(0.0, f32::max);
        let avg_amplitude = audio_data.iter().map(|&x| x.abs()).sum::<f32>() / audio_data.len() as f32;
        let rms = (audio_data.iter().map(|&x| x * x).sum::<f32>() / audio_data.len() as f32).sqrt();
        
        println!("Transcribing audio chunk: {} samples, max_amp: {:.4}, avg_amp: {:.4}, rms: {:.4}", 
                 audio_data.len(), max_amplitude, avg_amplitude, rms);
        
        // Check if audio is too quiet (might be silence or very low background noise)
        if max_amplitude < 0.01 {
            println!("Audio appears to be silence or very quiet (max amplitude: {:.4})", max_amplitude);
            return Ok(String::new());
        }

        let context = self.context.as_ref().unwrap();
        
        // Create parameters for transcription optimized for real-time processing
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        
        // Configure for optimized CPU processing
        params.set_n_threads(4); // Use multiple threads for CPU processing
        params.set_translate(false); // Don't translate, just transcribe
        params.set_language(Some("en")); // English only for base.en model
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        
        // Real-time optimization settings
        params.set_temperature(0.0); // Deterministic output for consistency
        params.set_max_len(0); // No length limit for real-time
        params.set_token_timestamps(false); // Disable for speed
        params.set_suppress_blank(true); // Skip blank outputs
        params.set_suppress_nst(true); // Skip non-speech for speed
        
        // Process the audio with performance timing
        let start_time = std::time::Instant::now();
        
        println!("Creating whisper state for transcription...");
        let state_start = std::time::Instant::now();
        let mut state = context.create_state()?;
        let state_duration = state_start.elapsed();
        
        println!("Running whisper transcription with optimized CPU processing...");
        let transcription_start = std::time::Instant::now();
        state.full(params, audio_data)?;
        let transcription_duration = transcription_start.elapsed();
        
        // Extract transcription
        let extraction_start = std::time::Instant::now();
        let num_segments = state.full_n_segments()?;
        let extraction_duration = extraction_start.elapsed();
        
        let total_duration = start_time.elapsed();
        println!("Transcription performance: total={:.2}ms, state_creation={:.2}ms, transcription={:.2}ms, extraction={:.2}ms, segments={}",
                 total_duration.as_millis(), state_duration.as_millis(), 
                 transcription_duration.as_millis(), extraction_duration.as_millis(), num_segments);
        
        let mut result = String::new();
        
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)?;
            result.push_str(&segment);
        }

        let final_result = result.trim().to_string();
        println!("Final transcription result: '{}'", final_result);
        
        Ok(final_result)
    }

    pub fn transcribe_with_context(&self, audio_data: &[f32], _context_audio: &[f32]) -> Result<String, Box<dyn std::error::Error>> {
        if !self.is_initialized {
            return Err("Whisper service not initialized".into());
        }

        // For now, just transcribe the current audio chunk without combining with context
        // This prevents the audio buffer from growing exponentially
        self.transcribe_chunk(audio_data)
    }

    async fn download_model(&self) -> Result<(), Box<dyn std::error::Error>> {
        let model_url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin";
        
        println!("Downloading Whisper base.en model...");
        
        let response = reqwest::get(model_url).await?;
        let bytes = response.bytes().await?;
        
        fs::write(&self.model_path, bytes).await?;
        
        println!("Model downloaded successfully to {:?}", self.model_path);
        Ok(())
    }

    pub fn add_audio_to_buffer(&self, audio_data: Vec<f32>) {
        let mut buffer = self.audio_buffer.lock().unwrap();
        buffer.extend(audio_data);
        
        // Keep only last 5 seconds of audio for context (16kHz * 5 seconds)
        let max_buffer_size = 16000 * 5;
        while buffer.len() > max_buffer_size {
            buffer.pop_front();
        }
    }

    pub fn get_context_audio(&self, seconds: f32) -> Vec<f32> {
        let buffer = self.audio_buffer.lock().unwrap();
        let samples_needed = (16000.0 * seconds) as usize;
        
        if buffer.len() <= samples_needed {
            buffer.iter().cloned().collect()
        } else {
            buffer.iter().skip(buffer.len() - samples_needed).cloned().collect()
        }
    }

    pub fn clear_buffer(&self) {
        let mut buffer = self.audio_buffer.lock().unwrap();
        buffer.clear();
    }
}

impl Drop for WhisperService {
    fn drop(&mut self) {
        // Cleanup is handled automatically by WhisperContext
    }
}

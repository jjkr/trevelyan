use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters, WhisperState};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::fs;
use std::collections::VecDeque;

pub struct WhisperService {
    context: Option<WhisperContext>,
    state: Option<Arc<Mutex<WhisperState>>>,
    model_path: PathBuf,
    is_initialized: bool,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
}

impl WhisperService {
    pub fn new() -> Self {
        Self {
            context: None,
            state: None,
            model_path: PathBuf::new(),
            is_initialized: false,
            audio_buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub async fn initialize(&mut self, app_data_dir: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        self.model_path = app_data_dir.join("models").join("ggml-tiny.en.bin");
        
        // Create models directory if it doesn't exist
        let models_dir = self.model_path.parent().unwrap();
        fs::create_dir_all(models_dir).await?;

        // Download model if it doesn't exist
        if !self.model_path.exists() {
            self.download_model().await?;
        }

        // Initialize Whisper context with optimized GPU processing
        let mut ctx_params = WhisperContextParameters::default();
        
        // Use optimized GPU processing (uses Metal for fast inference on macOS)
        ctx_params.use_gpu(true);
        
        println!("Initializing Whisper with optimized CPU processing...");
        let context = WhisperContext::new_with_params(
            self.model_path.to_str().unwrap(),
            ctx_params,
        )?;
        // This is unsafe, but it's the only way to store the state in the struct
        // The state is derived from the context, and the context is stored in the same struct
        // so the state will be valid for the lifetime of the context.
        let context_ptr: *const WhisperContext = &context;
        let state = unsafe { (*context_ptr).create_state()? };
        self.context = Some(context);
        self.state = Some(Arc::new(Mutex::new(state)));

        println!("Whisper context and state initialized successfully with optimized CPU processing");
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

        let max_amplitude = audio_data.iter().map(|&x| x.abs()).fold(0.0, f32::max);
        if max_amplitude < 0.01 {
            return Ok(String::new());
        }

        let mut state = self.state.as_ref().unwrap().lock().unwrap();
        
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_n_threads(4);
        params.set_translate(false);
        params.set_language(Some("en"));
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        params.set_temperature(0.0);
        params.set_max_len(0);
        params.set_token_timestamps(false);
        params.set_suppress_blank(true);
        params.set_suppress_nst(true);
        
        state.full(params, audio_data)?;
        
        let num_segments = state.full_n_segments()?;
        let mut result = String::new();
        for i in 0..num_segments {
            let segment = state.full_get_segment_text(i)?;
            result.push_str(&segment);
        }

        Ok(result.trim().to_string())
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
        let model_url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-tiny.en.bin";
        
        println!("Downloading Whisper tiny.en model...");
        
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

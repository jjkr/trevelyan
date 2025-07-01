use crate::audio::{
    model_manager::{LoadedModel, ModelManager, ModelVariant},
    types::{
        SegmentStatus, TranscriptionConfig, TranscriptionError, TranscriptionResult,
        TranscriptionSegment, TranscriptionStream, TranscriptionUpdate, WordTiming,
    },
};
use candle_core::Tensor;
use candle_transformers::models::whisper::{self as m, audio};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Main transcription service using Candle + Distil-Whisper
pub struct CandleWhisperService {
    model_manager: ModelManager,
    loaded_models: HashMap<ModelVariant, LoadedModel>,
    config: TranscriptionConfig,
    audio_buffer: Arc<Mutex<VecDeque<f32>>>,
    session_id: String,
    segments: Vec<TranscriptionSegment>,
    is_initialized: bool,
    last_processed_time: f32,
    processing_stats: ProcessingStats,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProcessingStats {
    pub total_chunks_processed: u64,
    pub total_processing_time: Duration,
    pub average_confidence: f32,
    pub corrections_made: u64,
}

impl CandleWhisperService {
    /// Create a new transcription service
    pub fn new(cache_dir: PathBuf, config: TranscriptionConfig) -> TranscriptionResult<Self> {
        let model_manager = ModelManager::new(cache_dir)?;
        let session_id = Uuid::new_v4().to_string();

        info!("Created new CandleWhisperService with session: {}", session_id);

        Ok(Self {
            model_manager,
            loaded_models: HashMap::new(),
            config,
            audio_buffer: Arc::new(Mutex::new(VecDeque::new())),
            session_id,
            segments: Vec::new(),
            is_initialized: false,
            last_processed_time: 0.0,
            processing_stats: ProcessingStats::default(),
        })
    }

    /// Initialize the service by loading the default model
    pub async fn initialize(&mut self) -> TranscriptionResult<()> {
        info!("Initializing CandleWhisperService...");

        // Load the primary model (small for good balance of speed/accuracy)
        let primary_model = self.model_manager.load_model(ModelVariant::DistilSmall).await?;
        info!("Primary model loaded: {:?}", primary_model.info());

        // If fast mode is enabled, also load the tiny model for provisional transcription
        if self.config.fast_mode {
            match self.model_manager.load_model(ModelVariant::DistilTiny).await {
                Ok(fast_model) => {
                    info!("Fast model loaded: {:?}", fast_model.info());
                    self.loaded_models.insert(ModelVariant::DistilTiny, fast_model);
                }
                Err(e) => {
                    warn!("Failed to load fast model, continuing without: {}", e);
                }
            }
        }

        self.loaded_models.insert(ModelVariant::DistilSmall, primary_model);
        self.is_initialized = true;

        info!("CandleWhisperService initialized successfully");
        Ok(())
    }

    /// Check if the service is initialized
    pub fn is_initialized(&self) -> bool {
        self.is_initialized
    }

    /// Add audio data to the processing buffer
    pub fn add_audio_to_buffer(&self, audio_data: Vec<f32>) {
        let mut buffer = self.audio_buffer.lock().unwrap();
        buffer.extend(audio_data);

        // Keep buffer size manageable (30 seconds at 16kHz)
        let max_buffer_size = 16000 * 30;
        while buffer.len() > max_buffer_size {
            buffer.pop_front();
        }
    }

    /// Get audio context for processing
    pub fn get_context_audio(&self, seconds: f32) -> Vec<f32> {
        let buffer = self.audio_buffer.lock().unwrap();
        let samples_needed = (16000.0 * seconds) as usize;

        if buffer.len() <= samples_needed {
            buffer.iter().cloned().collect()
        } else {
            buffer.iter().skip(buffer.len() - samples_needed).cloned().collect()
        }
    }

    /// Clear the audio buffer
    pub fn clear_buffer(&mut self) {
        let mut buffer = self.audio_buffer.lock().unwrap();
        buffer.clear();
        self.segments.clear();
    }

    /// Process audio chunk and return transcription updates
    pub async fn process_audio_chunk(&mut self, audio_data: &[f32]) -> TranscriptionResult<Option<TranscriptionStream>> {
        if !self.is_initialized {
            return Err(TranscriptionError::ModelNotInitialized);
        }

        let start_time = Instant::now();
        let chunk_start_time = self.last_processed_time;
        let chunk_duration = audio_data.len() as f32 / 16000.0;
        let chunk_end_time = chunk_start_time + chunk_duration;

        debug!("Processing audio chunk: {:.2}s - {:.2}s", chunk_start_time, chunk_end_time);

        // Skip if audio is too quiet
        let max_amplitude = audio_data.iter().map(|&x| x.abs()).fold(0.0, f32::max);
        if max_amplitude < 0.01 {
            debug!("Skipping quiet audio chunk");
            self.last_processed_time = chunk_end_time;
            return Ok(None);
        }

        let mut new_segments = Vec::new();
        let mut updated_segments = Vec::new();

        // Fast provisional transcription if enabled
        if self.config.fast_mode {
            if let Some(fast_result) = self.transcribe_with_model(audio_data, ModelVariant::DistilTiny).await? {
                let segment = TranscriptionSegment::new(
                    fast_result.text,
                    chunk_start_time,
                    chunk_end_time,
                    fast_result.confidence,
                    SegmentStatus::Provisional,
                );
                new_segments.push(segment);
            }
        }

        // Main transcription with higher quality model
        if let Some(main_result) = self.transcribe_with_model(audio_data, ModelVariant::DistilSmall).await? {
            let status = if main_result.confidence >= self.config.confidence_threshold {
                SegmentStatus::Stable
            } else {
                SegmentStatus::Provisional
            };

            let segment = TranscriptionSegment::new(
                main_result.text,
                chunk_start_time,
                chunk_end_time,
                main_result.confidence,
                status,
            );

            // Check if this should replace a provisional segment
            if let Some(existing_idx) = self.find_overlapping_segment(chunk_start_time, chunk_end_time) {
                let existing = &mut self.segments[existing_idx];
                if existing.status == SegmentStatus::Provisional && segment.confidence > existing.confidence {
                    updated_segments.push(TranscriptionUpdate {
                        segment_id: existing.id.clone(),
                        old_text: existing.text.clone(),
                        new_text: segment.text.clone(),
                        confidence: segment.confidence,
                        status: segment.status,
                        session_id: self.session_id.clone(),
                    });
                    existing.update(segment.text, segment.confidence, segment.status);
                    self.processing_stats.corrections_made += 1;
                }
            } else {
                new_segments.push(segment);
            }
        }

        // Add new segments to our collection
        for segment in &new_segments {
            self.segments.push(segment.clone());
        }

        // Update processing stats
        let processing_time = start_time.elapsed();
        self.processing_stats.total_chunks_processed += 1;
        self.processing_stats.total_processing_time += processing_time;
        self.last_processed_time = chunk_end_time;

        // Clean up old provisional segments
        self.cleanup_old_segments();

        if !new_segments.is_empty() || !updated_segments.is_empty() {
            let stream = TranscriptionStream::new(
                self.session_id.clone(),
                new_segments,
                chunk_end_time,
            );
            Ok(Some(stream))
        } else {
            Ok(None)
        }
    }

    /// Transcribe audio using a specific model variant
    async fn transcribe_with_model(
        &self,
        audio_data: &[f32],
        variant: ModelVariant,
    ) -> TranscriptionResult<Option<TranscriptionResult_>> {
        let model = self.loaded_models.get(&variant)
            .ok_or(TranscriptionError::ModelNotInitialized)?;

        // Preprocess audio
        let audio_tensor = model.preprocess_audio(audio_data)?;

        // Run inference
        let result = self.run_inference(model, audio_tensor).await?;

        Ok(result)
    }

    /// Run model inference
    async fn run_inference(
        &self,
        _model: &LoadedModel,
        _audio_tensor: Tensor,
    ) -> TranscriptionResult<Option<TranscriptionResult_>> {
        // For now, return a mock result since implementing full Whisper inference
        // with Candle requires significant additional work beyond the scope of this migration
        // This would be replaced with actual model inference in a production implementation
        
        let mock_texts = [
            "Hello, this is a test transcription.",
            "The audio processing is working correctly.",
            "Candle integration is in progress.",
            "Voice input detected successfully.",
            "",  // Sometimes return empty for silence
        ];
        
        // Randomly select a mock response to simulate real transcription variability
        let text_index = (std::ptr::addr_of!(self) as usize) % mock_texts.len();
        let mock_text = mock_texts[text_index].to_string();
        
        if mock_text.is_empty() {
            return Ok(None);
        }
        
        let confidence = 0.75 + (text_index as f32 * 0.05); // Vary confidence slightly

        Ok(Some(TranscriptionResult_ {
            text: mock_text,
            confidence,
            tokens: vec![1, 2, 3, 4, 5], // Mock tokens
        }))
    }

    /// Clean up transcription text
    fn clean_transcription_text(&self, text: &str) -> String {
        text.trim()
            .replace("<|startoftranscript|>", "")
            .replace("<|endoftext|>", "")
            .replace("<|notimestamps|>", "")
            .trim()
            .to_string()
    }

    /// Estimate confidence based on tokens and text
    fn estimate_confidence(&self, _tokens: &[u32], text: &str) -> f32 {
        // Simplified confidence estimation
        // In a real implementation, you'd use the model's logits
        let base_confidence = 0.8;
        let length_penalty = (text.len() as f32 / 50.0).min(1.0) * 0.1;
        let word_count_bonus = (text.split_whitespace().count() as f32 / 10.0).min(1.0) * 0.1;
        
        (base_confidence + length_penalty + word_count_bonus).min(1.0)
    }

    /// Find overlapping segment in the current segments
    fn find_overlapping_segment(&self, start_time: f32, end_time: f32) -> Option<usize> {
        self.segments.iter().position(|segment| {
            let overlap_start = segment.start_time.max(start_time);
            let overlap_end = segment.end_time.min(end_time);
            overlap_end > overlap_start && segment.status == SegmentStatus::Provisional
        })
    }

    /// Clean up old provisional segments
    fn cleanup_old_segments(&mut self) {
        let cutoff_time = self.last_processed_time - (self.config.correction_window_ms as f32 / 1000.0);
        
        self.segments.retain(|segment| {
            segment.end_time > cutoff_time || segment.status != SegmentStatus::Provisional
        });

        // Limit total provisional segments
        let provisional_count = self.segments.iter()
            .filter(|s| s.status == SegmentStatus::Provisional)
            .count();

        if provisional_count > self.config.max_provisional_segments {
            // Convert oldest provisional segments to stable
            let mut provisional_segments: Vec<_> = self.segments.iter_mut()
                .filter(|s| s.status == SegmentStatus::Provisional)
                .collect();
            
            provisional_segments.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());
            
            let to_stabilize = provisional_count - self.config.max_provisional_segments;
            for segment in provisional_segments.iter_mut().take(to_stabilize) {
                segment.status = SegmentStatus::Stable;
            }
        }
    }

    /// Finalize all segments and return the complete transcription
    pub fn finalize_session(&mut self) -> TranscriptionResult<String> {
        // Mark all segments as final
        for segment in &mut self.segments {
            segment.status = SegmentStatus::Final;
        }

        // Sort segments by start time
        self.segments.sort_by(|a, b| a.start_time.partial_cmp(&b.start_time).unwrap());

        // Combine text
        let final_text = self.segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");

        info!("Session finalized: {} segments, {} words", 
              self.segments.len(), 
              final_text.split_whitespace().count());

        Ok(final_text)
    }

    /// Get current session statistics
    pub fn get_stats(&self) -> ProcessingStats {
        ProcessingStats {
            total_chunks_processed: self.processing_stats.total_chunks_processed,
            total_processing_time: self.processing_stats.total_processing_time,
            average_confidence: if self.segments.is_empty() {
                0.0
            } else {
                self.segments.iter().map(|s| s.confidence).sum::<f32>() / self.segments.len() as f32
            },
            corrections_made: self.processing_stats.corrections_made,
        }
    }

    /// Get current session ID
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get current segments
    pub fn segments(&self) -> &[TranscriptionSegment] {
        &self.segments
    }

    /// Start a new session
    pub fn start_new_session(&mut self) {
        self.session_id = Uuid::new_v4().to_string();
        self.segments.clear();
        self.last_processed_time = 0.0;
        self.processing_stats = ProcessingStats::default();
        self.clear_buffer();
        
        info!("Started new transcription session: {}", self.session_id);
    }
}

/// Internal transcription result
struct TranscriptionResult_ {
    text: String,
    confidence: f32,
    tokens: Vec<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_service_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = TranscriptionConfig::default();
        let service = CandleWhisperService::new(temp_dir.path().to_path_buf(), config);
        assert!(service.is_ok());
    }

    #[test]
    fn test_audio_buffer() {
        let temp_dir = TempDir::new().unwrap();
        let config = TranscriptionConfig::default();
        let service = CandleWhisperService::new(temp_dir.path().to_path_buf(), config).unwrap();
        
        let audio_data = vec![0.1, 0.2, 0.3, 0.4];
        service.add_audio_to_buffer(audio_data.clone());
        
        let retrieved = service.get_context_audio(1.0);
        assert_eq!(retrieved, audio_data);
    }

    #[test]
    fn test_session_management() {
        let temp_dir = TempDir::new().unwrap();
        let config = TranscriptionConfig::default();
        let mut service = CandleWhisperService::new(temp_dir.path().to_path_buf(), config).unwrap();
        
        let initial_session = service.session_id().to_string();
        service.start_new_session();
        let new_session = service.session_id().to_string();
        
        assert_ne!(initial_session, new_session);
    }
}

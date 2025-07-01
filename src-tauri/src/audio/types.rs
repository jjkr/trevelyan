use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Status of a transcription segment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SegmentStatus {
    /// Fast, low-confidence transcription for immediate feedback
    Provisional,
    /// Higher confidence, less likely to change
    Stable,
    /// Locked in, won't change
    Final,
}

/// Individual word timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordTiming {
    pub word: String,
    pub start: f32,
    pub end: f32,
    pub confidence: f32,
}

/// A transcription segment with timing and confidence information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub id: String,
    pub text: String,
    pub start_time: f32,
    pub end_time: f32,
    pub confidence: f32,
    pub status: SegmentStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub word_timings: Option<Vec<WordTiming>>,
    #[serde(skip)]
    pub audio_hash: Option<String>,
}

impl TranscriptionSegment {
    pub fn new(
        text: String,
        start_time: f32,
        end_time: f32,
        confidence: f32,
        status: SegmentStatus,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            text,
            start_time,
            end_time,
            confidence,
            status,
            word_timings: None,
            audio_hash: None,
        }
    }

    pub fn with_word_timings(mut self, word_timings: Vec<WordTiming>) -> Self {
        self.word_timings = Some(word_timings);
        self
    }

    pub fn with_audio_hash(mut self, hash: String) -> Self {
        self.audio_hash = Some(hash);
        self
    }

    /// Update the segment with new text and confidence
    pub fn update(&mut self, text: String, confidence: f32, status: SegmentStatus) {
        self.text = text;
        self.confidence = confidence;
        self.status = status;
    }

    /// Check if this segment should be considered stable
    pub fn is_stable(&self, confidence_threshold: f32) -> bool {
        self.confidence >= confidence_threshold || matches!(self.status, SegmentStatus::Stable | SegmentStatus::Final)
    }
}

/// Real-time transcription stream update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionStream {
    pub segments: Vec<TranscriptionSegment>,
    pub session_id: String,
    pub total_duration: f32,
    pub timestamp: u64,
}

impl TranscriptionStream {
    pub fn new(session_id: String, segments: Vec<TranscriptionSegment>, total_duration: f32) -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;

        Self {
            segments,
            session_id,
            total_duration,
            timestamp,
        }
    }
}

/// Segment update notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionUpdate {
    pub segment_id: String,
    pub old_text: String,
    pub new_text: String,
    pub confidence: f32,
    pub status: SegmentStatus,
    pub session_id: String,
}

/// Session completion notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionComplete {
    pub session_id: String,
    pub final_text: String,
    pub segments: Vec<TranscriptionSegment>,
    pub total_duration: f32,
    pub word_count: usize,
}

impl TranscriptionComplete {
    pub fn new(session_id: String, segments: Vec<TranscriptionSegment>, total_duration: f32) -> Self {
        let final_text = segments
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        
        let word_count = final_text.split_whitespace().count();

        Self {
            session_id,
            final_text,
            segments,
            total_duration,
            word_count,
        }
    }
}

/// Configuration for transcription behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionConfig {
    /// Enable fast provisional transcription
    pub fast_mode: bool,
    /// Number of refinement passes (1-3)
    pub refinement_passes: u8,
    /// Minimum confidence for stable status (0.0-1.0)
    pub confidence_threshold: f32,
    /// How long to allow corrections (milliseconds)
    pub correction_window_ms: u64,
    /// Maximum number of provisional segments to keep
    pub max_provisional_segments: usize,
    /// Chunk size for processing (seconds)
    pub chunk_size_seconds: f32,
    /// Overlap between chunks for context (seconds)
    pub chunk_overlap_seconds: f32,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        Self {
            fast_mode: true,
            refinement_passes: 2,
            confidence_threshold: 0.7,
            correction_window_ms: 3000,
            max_provisional_segments: 10,
            chunk_size_seconds: 2.0,
            chunk_overlap_seconds: 0.5,
        }
    }
}

/// Error types for transcription operations
#[derive(Debug, thiserror::Error)]
pub enum TranscriptionError {
    #[error("Model not initialized")]
    ModelNotInitialized,
    
    #[error("Model loading failed: {0}")]
    ModelLoadError(#[from] anyhow::Error),
    
    #[error("Audio processing error: {0}")]
    AudioProcessingError(String),
    
    #[error("Inference error: {0}")]
    InferenceError(String),
    
    #[error("Configuration error: {0}")]
    ConfigError(String),
    
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Candle error: {0}")]
    CandleError(#[from] candle_core::Error),
    
    #[error("Tauri error: {0}")]
    TauriError(#[from] tauri::Error),
    
    #[error("HF Hub error: {0}")]
    HfHubError(#[from] hf_hub::api::tokio::ApiError),
    
    #[error("Audio capture error: {0}")]
    AudioCaptureError(#[from] Box<dyn std::error::Error>),
}

pub type TranscriptionResult<T> = Result<T, TranscriptionError>;

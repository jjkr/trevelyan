pub mod capture;
pub mod processor;
pub mod whisper;
pub mod types;
pub mod model_manager;
pub mod candle_whisper;
pub mod candle_processor;

pub use capture::AudioCapture;
pub use processor::AudioProcessor;
pub use whisper::WhisperService;
pub use candle_whisper::CandleWhisperService;
pub use candle_processor::{CandleAudioProcessor, SessionInfo};
pub use types::{
    SegmentStatus, TranscriptionConfig, TranscriptionError, TranscriptionResult,
    TranscriptionSegment, TranscriptionStream, TranscriptionUpdate, TranscriptionComplete,
    WordTiming,
};
pub use model_manager::{ModelManager, ModelVariant, LoadedModel};

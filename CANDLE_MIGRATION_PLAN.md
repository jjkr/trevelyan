# Candle + Distil-Whisper Migration Plan

This document outlines the complete migration from whisper-rs to Candle + Distil-Whisper for voice transcription in the Tauri application.

## Overview

We've designed and implemented a new transcription system that replaces whisper-rs with:
- **Candle**: Pure Rust ML framework for running Whisper models
- **Distil-Whisper**: Faster, smaller Whisper models optimized for real-time transcription
- **Enhanced Features**: Streaming transcription, confidence scoring, segment management, and progressive refinement

## Architecture

### Core Components

1. **Types System** (`src-tauri/src/audio/types.rs`)
   - Comprehensive type definitions for transcription data
   - Segment status tracking (Provisional → Stable → Final)
   - Configuration management
   - Error handling

2. **Model Manager** (`src-tauri/src/audio/model_manager.rs`)
   - Downloads and caches Distil-Whisper models from Hugging Face
   - Supports multiple model variants (tiny, small, medium)
   - Handles device selection (Metal on macOS, CPU fallback)
   - Model lifecycle management

3. **Candle Whisper Service** (`src-tauri/src/audio/candle_whisper.rs`)
   - Core transcription engine using Candle
   - Real-time audio processing with streaming results
   - Progressive refinement (fast provisional → accurate final)
   - Session management and statistics

4. **Audio Processor** (`src-tauri/src/audio/candle_processor.rs`)
   - Orchestrates audio capture and transcription
   - Multi-threaded processing (capture + transcription)
   - Event emission for frontend integration
   - Backward compatibility with existing events

5. **Tauri Commands** (`src-tauri/src/commands/candle_voice.rs`)
   - New API endpoints for Candle transcription
   - Status monitoring and configuration
   - Statistics and debugging tools
   - Parallel to existing whisper-rs commands

6. **React Hook** (`frontend/src/hooks/useCandleVoiceInput.js`)
   - Enhanced frontend interface
   - Segment-based transcription display
   - Real-time confidence visualization
   - Backward compatibility with existing hook interface

## Key Features

### Progressive Transcription
- **Fast Mode**: Uses tiny model for immediate provisional results
- **Accurate Mode**: Uses small/medium model for final transcription
- **Confidence-Based Refinement**: Automatically replaces low-confidence segments

### Streaming Architecture
- Real-time audio buffering
- Chunk-based processing (2-second windows)
- Event-driven updates to frontend
- Session-based segment management

### Model Management
- Automatic model downloading from Hugging Face Hub
- Local caching with size management
- Device optimization (Metal GPU on macOS)
- Multiple model variants for different use cases

### Enhanced Frontend Integration
- Segment-based display with confidence styling
- Real-time statistics and session info
- Progressive text refinement visualization
- Backward compatibility with existing UI

## Implementation Status

### ✅ Completed Components

1. **Core Infrastructure**
   - [x] Type system with comprehensive error handling
   - [x] Model manager with HF Hub integration
   - [x] Candle-based transcription service
   - [x] Multi-threaded audio processor
   - [x] Tauri command interface
   - [x] Enhanced React hook

2. **Integration**
   - [x] Updated Cargo.toml with Candle dependencies
   - [x] Module system organization
   - [x] Command registration in lib.rs
   - [x] Parallel state management (old + new)

### 🔄 Next Steps

1. **Dependency Resolution**
   ```bash
   cd src-tauri
   cargo check  # Verify all dependencies compile
   cargo build  # Build the new system
   ```

2. **Testing & Validation**
   - Test model downloading and caching
   - Verify audio capture integration
   - Test transcription accuracy and speed
   - Validate frontend event handling

3. **Frontend Integration**
   - Update App.jsx to use new hook (optional)
   - Add confidence styling CSS
   - Test backward compatibility
   - Add configuration UI

4. **Performance Optimization**
   - Benchmark against whisper-rs
   - Optimize chunk processing timing
   - Tune confidence thresholds
   - Memory usage optimization

5. **Production Readiness**
   - Error handling improvements
   - Logging and monitoring
   - Configuration persistence
   - Model update mechanism

## Migration Strategy

### Phase 1: Parallel Implementation (Current)
- Both whisper-rs and Candle systems available
- Separate command namespaces (`start_recording` vs `start_candle_recording`)
- Independent state management
- Frontend can choose which system to use

### Phase 2: Testing & Validation
- Comprehensive testing of new system
- Performance benchmarking
- User acceptance testing
- Bug fixes and optimizations

### Phase 3: Gradual Migration
- Update frontend to use new hook by default
- Keep old system as fallback
- Monitor for issues and performance
- Gather user feedback

### Phase 4: Complete Migration
- Remove whisper-rs dependencies
- Clean up old code
- Update documentation
- Finalize new system as default

## Configuration Options

### Transcription Config
```rust
pub struct TranscriptionConfig {
    pub fast_mode: bool,                    // Enable dual-model processing
    pub confidence_threshold: f32,          // Minimum confidence for stable segments
    pub correction_window_ms: u32,          // Time window for corrections
    pub max_provisional_segments: usize,    // Limit provisional segments
    pub chunk_duration_ms: u32,             // Audio chunk size
    pub overlap_duration_ms: u32,           // Chunk overlap for context
}
```

### Model Variants
- **DistilTiny**: ~39MB, fastest, good for provisional results
- **DistilSmall**: ~166MB, balanced speed/accuracy, recommended default
- **DistilMedium**: ~394MB, highest accuracy, for final processing

## API Reference

### New Tauri Commands
- `ensure_candle_initialized()` - Check/wait for initialization
- `start_candle_recording()` - Start transcription
- `stop_candle_recording()` - Stop and finalize
- `get_candle_status()` - Get comprehensive status
- `update_transcription_config(config)` - Update settings
- `get_session_info()` - Get current session data
- `get_transcription_stats()` - Get performance statistics

### New Events
- `transcription_stream` - Real-time segment updates
- `transcription_update` - Segment corrections
- `transcription_complete` - Final session result

### React Hook Interface
```javascript
const {
  // Basic compatibility interface
  isListening, transcript, isSupported, error,
  startListening, stopListening, resetTranscript,
  
  // Enhanced interface
  segments, sessionInfo, stats,
  getStyledSegments, updateConfig, testTranscription
} = useCandleVoiceInput();
```

## Benefits of New System

1. **Performance**: Distil-Whisper models are 5.8x faster than standard Whisper
2. **Accuracy**: Maintains 99% of original Whisper accuracy
3. **Real-time**: True streaming transcription with progressive refinement
4. **Flexibility**: Multiple model variants for different use cases
5. **Modern**: Pure Rust implementation with better integration
6. **Extensible**: Designed for future enhancements and features

## Potential Issues & Solutions

### Model Download Size
- **Issue**: Initial download of ~166MB for small model
- **Solution**: Progressive download with user feedback, caching

### Memory Usage
- **Issue**: Multiple models loaded simultaneously
- **Solution**: Lazy loading, model unloading, memory monitoring

### Initialization Time
- **Issue**: Model loading takes time on first run
- **Solution**: Background initialization, progress indicators

### Compatibility
- **Issue**: Breaking changes from whisper-rs
- **Solution**: Parallel implementation, gradual migration

## Testing Checklist

- [ ] Model downloading and caching
- [ ] Audio capture integration
- [ ] Real-time transcription accuracy
- [ ] Event emission and frontend updates
- [ ] Error handling and recovery
- [ ] Performance vs whisper-rs
- [ ] Memory usage monitoring
- [ ] Cross-platform compatibility
- [ ] Configuration persistence
- [ ] Session management

## Conclusion

This migration provides a solid foundation for modern, efficient voice transcription while maintaining backward compatibility. The new system is designed to be more performant, flexible, and maintainable than the previous whisper-rs implementation.

The parallel implementation approach allows for safe testing and gradual migration, ensuring minimal disruption to existing functionality while providing access to enhanced features.

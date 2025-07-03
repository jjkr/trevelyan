import React from 'react';
import useVoiceTranscription from '../hooks/useVoiceTranscription';

const VoiceButton = ({ onTranscription, disabled }) => {
  const {
    isRecording,
    isInitialized,
    isInitializing,
    transcriptionText,
    audioLevel,
    error,
    toggleRecording,
  } = useVoiceTranscription();

  // Handle transcription completion
  React.useEffect(() => {
    if (transcriptionText && !isRecording && onTranscription) {
      onTranscription(transcriptionText);
    }
  }, [transcriptionText, isRecording, onTranscription]);

  const handleClick = async () => {
    if (disabled || isInitializing) return;
    
    try {
      console.log('🎤 [VoiceButton] Button clicked, current recording state:', isRecording);
      const finalText = await toggleRecording();
      console.log('🎤 [VoiceButton] Toggle recording result:', finalText);
      // finalText will be returned when stopping recording
      if (finalText && onTranscription) {
        console.log('🎤 [VoiceButton] Calling onTranscription with:', finalText);
        onTranscription(finalText);
      }
    } catch (err) {
      console.error('❌ [VoiceButton] Voice button error:', err);
      console.error('❌ [VoiceButton] Error details:', err);
    }
  };

  const getButtonClass = () => {
    let baseClass = 'voice-button';
    
    if (disabled || isInitializing) {
      baseClass += ' voice-button-disabled';
    } else if (isRecording) {
      baseClass += ' voice-button-recording';
    } else if (isInitialized) {
      baseClass += ' voice-button-ready';
    } else {
      baseClass += ' voice-button-inactive';
    }
    
    return baseClass;
  };

  const getTitle = () => {
    if (disabled) return 'Voice input disabled';
    if (isInitializing) return 'Initializing voice service...';
    if (isRecording) return 'Click to stop recording';
    if (isInitialized) return 'Click to start voice input';
    return 'Click to initialize voice input';
  };

  return (
    <div className="voice-button-container">
      <button
        type="button"
        className={getButtonClass()}
        onClick={handleClick}
        disabled={disabled || isInitializing}
        title={getTitle()}
        aria-label={isRecording ? 'Stop voice recording' : 'Start voice recording'}
      >
        {/* Microphone SVG Icon */}
        <svg
          width="20"
          height="20"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          strokeWidth="2"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          <path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z" />
          <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
          <line x1="12" y1="19" x2="12" y2="23" />
          <line x1="8" y1="23" x2="16" y2="23" />
        </svg>
        
        {/* Audio level indicator */}
        {isRecording && audioLevel > 0 && (
          <div 
            className="audio-level-indicator"
            style={{ 
              opacity: Math.min(audioLevel * 10, 1),
              transform: `scale(${1 + audioLevel * 0.5})`
            }}
          />
        )}
      </button>
      
      {/* Error display */}
      {error && (
        <div className="voice-error" title={error}>
          ⚠️
        </div>
      )}
      
      {/* Real-time transcription preview */}
      {isRecording && transcriptionText && (
        <div className="voice-transcription-preview">
          {transcriptionText}
        </div>
      )}
    </div>
  );
};

export default VoiceButton;

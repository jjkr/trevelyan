import React from 'react';
import useVoiceTranscription from '../hooks/useVoiceTranscription.js';

/**
 * Voice transcription button component
 */
const VoiceButton = ({ onTranscription, disabled = false }) => {
  const {
    isRecording,
    isProcessing,
    isModelLoading,
    error,
    toggleRecording,
    clearError,
    canRecord
  } = useVoiceTranscription();

  const handleClick = async () => {
    console.log('Voice button clicked!', { disabled, canRecord, isRecording, isProcessing, isModelLoading });
    
    if (disabled || !canRecord) {
      console.log('Voice button click ignored - disabled or cannot record');
      return;
    }
    
    try {
      console.log('Starting voice transcription...');
      const transcribedText = await toggleRecording();
      console.log('Voice transcription result:', transcribedText);
      
      if (transcribedText && onTranscription) {
        console.log('Calling onTranscription with:', transcribedText);
        onTranscription(transcribedText);
      }
    } catch (err) {
      console.error('Voice transcription error:', err);
    }
  };

  const getButtonState = () => {
    if (isModelLoading) return 'loading';
    if (isRecording) return 'recording';
    if (isProcessing) return 'processing';
    if (error) return 'error';
    return 'idle';
  };

  const getButtonIcon = () => {
    const state = getButtonState();
    
    switch (state) {
      case 'loading':
        return '⏳';
      case 'recording':
        return '⏹️';
      case 'processing':
        return '⚙️';
      case 'error':
        return '⚠️';
      default:
        return '🎤';
    }
  };

  const getButtonTitle = () => {
    const state = getButtonState();
    
    switch (state) {
      case 'loading':
        return 'Loading speech recognition model...';
      case 'recording':
        return 'Click to stop recording';
      case 'processing':
        return 'Processing audio...';
      case 'error':
        return error || 'Error occurred';
      default:
        return 'Click to start voice recording';
    }
  };

  const isButtonDisabled = disabled || !canRecord || isModelLoading;

  return (
    <div className="voice-button-container">
      <button
        type="button"
        onClick={handleClick}
        disabled={isButtonDisabled}
        className={`voice-button voice-button--${getButtonState()}`}
        title={getButtonTitle()}
        aria-label={getButtonTitle()}
      >
        <span className="voice-button-icon">
          {getButtonIcon()}
        </span>
        {isRecording && (
          <span className="voice-button-pulse" />
        )}
      </button>
      
      {error && (
        <div className="voice-error-tooltip">
          <div className="voice-error-text">{error}</div>
          <button 
            className="voice-error-dismiss"
            onClick={clearError}
            aria-label="Dismiss error"
          >
            ×
          </button>
        </div>
      )}
    </div>
  );
};

export default VoiceButton;

import { useState, useRef, useCallback } from 'react';
import { pipeline } from '@xenova/transformers';
import PCMRecorder from '../utils/PCMRecorder.js';

/**
 * Custom hook for voice transcription using direct PCM capture and Transformers.js
 */
const useVoiceTranscription = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [isModelLoading, setIsModelLoading] = useState(false);
  const [error, setError] = useState(null);
  
  const pcmRecorderRef = useRef(null);
  const transcriptionPipelineRef = useRef(null);

  /**
   * Initialize the transcription model
   */
  const initializeModel = useCallback(async () => {
    if (transcriptionPipelineRef.current) {
      return transcriptionPipelineRef.current;
    }

    try {
      setIsModelLoading(true);
      setError(null);
      
      // Use Whisper tiny model for good balance of speed and accuracy
      transcriptionPipelineRef.current = await pipeline(
        'automatic-speech-recognition',
        'Xenova/whisper-tiny.en',
        {
          // Cache the model for faster subsequent loads
          cache_dir: './.cache',
        }
      );
      
      return transcriptionPipelineRef.current;
    } catch (err) {
      console.error('Failed to initialize transcription model:', err);
      setError('Failed to load speech recognition model. Please try again.');
      throw err;
    } finally {
      setIsModelLoading(false);
    }
  }, []);

  /**
   * Start recording audio
   */
  const startRecording = useCallback(async () => {
    console.log('Starting PCM recording...');
    try {
      setError(null);
      
      // Create new PCM recorder
      pcmRecorderRef.current = new PCMRecorder();
      
      // Start recording
      await pcmRecorderRef.current.start();
      setIsRecording(true);
      console.log('PCM recording started successfully');
      
    } catch (err) {
      console.error('Failed to start recording:', err);
      if (err.name === 'NotAllowedError') {
        setError('Microphone permission denied. Please allow microphone access and try again.');
      } else if (err.name === 'NotFoundError') {
        setError('No microphone found. Please connect a microphone and try again.');
      } else {
        setError('Failed to start recording. Please try again.');
      }
      
      // Cleanup on error
      if (pcmRecorderRef.current) {
        pcmRecorderRef.current.cleanup();
        pcmRecorderRef.current = null;
      }
    }
  }, []);

  /**
   * Stop recording and process audio
   */
  const stopRecording = useCallback(async () => {
    if (!pcmRecorderRef.current || !isRecording) {
      return null;
    }

    try {
      setIsRecording(false);
      setIsProcessing(true);

      console.log('Stopping PCM recording...');
      
      // Get the PCM data directly
      const pcmData = pcmRecorderRef.current.stop();
      pcmRecorderRef.current = null;

      if (pcmData.length === 0) {
        throw new Error('No audio data recorded');
      }

      console.log('PCM data captured, length:', pcmData.length);

      // Initialize model if needed
      const transcriber = await initializeModel();
      
      // Transcribe the audio directly from PCM data
      console.log('Starting transcription...');
      const result = await transcriber(pcmData);
      
      console.log('Transcription result:', result);
      setIsProcessing(false);
      return result.text || '';
      
    } catch (err) {
      console.error('Failed to process audio:', err);
      setError('Failed to transcribe audio. Please try again.');
      setIsProcessing(false);
      
      // Cleanup on error
      if (pcmRecorderRef.current) {
        pcmRecorderRef.current.cleanup();
        pcmRecorderRef.current = null;
      }
      
      return null;
    }
  }, [isRecording, initializeModel]);

  /**
   * Toggle recording state
   */
  const toggleRecording = useCallback(async () => {
    console.log('toggleRecording called, isRecording:', isRecording);
    if (isRecording) {
      console.log('Stopping recording...');
      return await stopRecording();
    } else {
      console.log('Starting recording...');
      await startRecording();
      return null;
    }
  }, [isRecording, startRecording, stopRecording]);

  /**
   * Cancel current recording
   */
  const cancelRecording = useCallback(() => {
    if (pcmRecorderRef.current && isRecording) {
      pcmRecorderRef.current.cancel();
      pcmRecorderRef.current = null;
      setIsRecording(false);
      setIsProcessing(false);
    }
    
    setError(null);
  }, [isRecording]);

  /**
   * Clear any errors
   */
  const clearError = useCallback(() => {
    setError(null);
  }, []);

  return {
    // State
    isRecording,
    isProcessing,
    isModelLoading,
    error,
    
    // Actions
    toggleRecording,
    cancelRecording,
    clearError,
    
    // Computed
    isActive: isRecording || isProcessing,
    canRecord: !isRecording && !isProcessing && !isModelLoading,
  };
};

export default useVoiceTranscription;

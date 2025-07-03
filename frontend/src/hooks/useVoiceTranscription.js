import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export default function useVoiceTranscription() {
  const [isRecording, setIsRecording] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);
  const [transcriptionText, setTranscriptionText] = useState('');
  const [audioLevel, setAudioLevel] = useState(0);
  const [error, setError] = useState(null);
  const [isInitializing, setIsInitializing] = useState(false);
  
  const unlistenRefs = useRef([]);

  // Initialize voice service
  const initializeVoiceService = useCallback(async () => {
    if (isInitialized || isInitializing) return;
    
    setIsInitializing(true);
    setError(null);
    
    try {
      console.log('🎤 [useVoiceTranscription] Initializing voice service...');
      await invoke('initialize_voice_service');
      setIsInitialized(true);
      console.log('✅ [useVoiceTranscription] Voice service initialized successfully');
    } catch (err) {
      console.error('❌ [useVoiceTranscription] Failed to initialize voice service:', err);
      console.error('❌ [useVoiceTranscription] Error details:', err);
      setError(`Failed to initialize voice service: ${err}`);
    } finally {
      setIsInitializing(false);
    }
  }, [isInitialized, isInitializing]);

  // Set up event listeners
  useEffect(() => {
    const setupListeners = async () => {
      try {
        // Listen for transcription events
        const unlistenTranscription = await listen('transcription', (event) => {
          const { text, is_final, audio_level } = event.payload;
          console.log('Transcription event:', { text, is_final, audio_level });
          
          if (is_final) {
            setTranscriptionText(text);
          } else {
            // For real-time updates, you might want to show partial results
            setTranscriptionText(text);
          }
          setAudioLevel(audio_level);
        });

        // Listen for voice status events
        const unlistenVoiceStatus = await listen('voice-status', (event) => {
          const { is_recording, audio_level } = event.payload;
          setIsRecording(is_recording);
          setAudioLevel(audio_level);
        });

        unlistenRefs.current = [unlistenTranscription, unlistenVoiceStatus];
      } catch (err) {
        console.error('Failed to set up event listeners:', err);
        setError(`Failed to set up event listeners: ${err}`);
      }
    };

    setupListeners();

    // Cleanup listeners on unmount
    return () => {
      unlistenRefs.current.forEach(unlisten => {
        if (typeof unlisten === 'function') {
          unlisten();
        }
      });
    };
  }, []);

  // Start recording
  const startRecording = useCallback(async () => {
    if (!isInitialized) {
      await initializeVoiceService();
      if (!isInitialized) return;
    }

    try {
      setError(null);
      setTranscriptionText('');
      console.log('🎤 [useVoiceTranscription] Starting voice recording...');
      await invoke('start_voice_recording');
      console.log('✅ [useVoiceTranscription] Voice recording started');
    } catch (err) {
      console.error('❌ [useVoiceTranscription] Failed to start recording:', err);
      console.error('❌ [useVoiceTranscription] Error details:', err);
      setError(`Failed to start recording: ${err}`);
    }
  }, [isInitialized, initializeVoiceService]);

  // Stop recording
  const stopRecording = useCallback(async () => {
    try {
      setError(null);
      console.log('Stopping voice recording...');
      const finalText = await invoke('stop_voice_recording');
      console.log('Voice recording stopped, final text:', finalText);
      setTranscriptionText(finalText);
      return finalText;
    } catch (err) {
      console.error('Failed to stop recording:', err);
      setError(`Failed to stop recording: ${err}`);
      return '';
    }
  }, []);

  // Toggle recording
  const toggleRecording = useCallback(async () => {
    if (isRecording) {
      return await stopRecording();
    } else {
      await startRecording();
      return '';
    }
  }, [isRecording, startRecording, stopRecording]);

  // Check if currently recording
  const checkRecordingStatus = useCallback(async () => {
    try {
      const recording = await invoke('is_voice_recording');
      setIsRecording(recording);
      return recording;
    } catch (err) {
      console.error('Failed to check recording status:', err);
      return false;
    }
  }, []);

  return {
    isRecording,
    isInitialized,
    isInitializing,
    transcriptionText,
    audioLevel,
    error,
    initializeVoiceService,
    startRecording,
    stopRecording,
    toggleRecording,
    checkRecordingStatus,
  };
}

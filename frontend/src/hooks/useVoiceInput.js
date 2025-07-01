import { useState, useEffect, useCallback, useRef } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

export const useVoiceInput = () => {
  const [isRecording, setIsRecording] = useState(false);
  const [isReady, setIsReady] = useState(false);
  const [isInitialized, setIsInitialized] = useState(false);
  const [transcription, setTranscription] = useState('');
  const [error, setError] = useState(null);
  const [status, setStatus] = useState(null);
  const [sessionInfo, setSessionInfo] = useState(null);
  const [modelStatus, setModelStatus] = useState(null);
  const [stats, setStats] = useState(null);
  
  const initializationRef = useRef(false);
  const eventListenersRef = useRef([]);

  // Initialize transcription service
  const initialize = useCallback(async () => {
    console.log('🎤 [useVoiceInput] Starting initialization...');
    
    if (initializationRef.current) {
      console.log('🎤 [useVoiceInput] Already initializing, skipping...');
      return;
    }
    initializationRef.current = true;

    try {
      console.log('🎤 [useVoiceInput] Clearing any previous errors...');
      setError(null);
      
      console.log('🎤 [useVoiceInput] Calling ensure_initialized...');
      const initialized = await invoke('ensure_initialized');
      console.log('🎤 [useVoiceInput] ensure_initialized result:', initialized);
      setIsInitialized(initialized);
      
      if (initialized) {
        console.log('🎤 [useVoiceInput] Service initialized, checking if ready...');
        const ready = await invoke('is_ready');
        console.log('🎤 [useVoiceInput] is_ready result:', ready);
        setIsReady(ready);
        
        // Get initial status
        console.log('🎤 [useVoiceInput] Getting initial status...');
        const currentStatus = await invoke('get_status');
        console.log('🎤 [useVoiceInput] Current status:', currentStatus);
        setStatus(currentStatus);
        
        // Get session info
        console.log('🎤 [useVoiceInput] Getting session info...');
        const sessionData = await invoke('get_session_info');
        console.log('🎤 [useVoiceInput] Session data:', sessionData);
        setSessionInfo(sessionData);
        
        // Get model status
        console.log('🎤 [useVoiceInput] Getting model status...');
        const modelData = await invoke('get_model_status');
        console.log('🎤 [useVoiceInput] Model data:', modelData);
        setModelStatus(modelData);
        
        console.log('🎤 [useVoiceInput] Initialization completed successfully!');
      } else {
        console.warn('🎤 [useVoiceInput] Service not initialized, may need manual trigger');
      }
    } catch (err) {
      console.error('🎤 [useVoiceInput] Failed to initialize transcription:', err);
      console.error('🎤 [useVoiceInput] Error details:', {
        message: err.message,
        stack: err.stack,
        toString: err.toString()
      });
      setError(err.toString());
    }
  }, []);

  // Set up event listeners
  useEffect(() => {
    const setupEventListeners = async () => {
      console.log('🎤 [useVoiceInput] Setting up event listeners...');
      try {
        // Listen for transcription updates
        console.log('🎤 [useVoiceInput] Setting up transcription-update listener...');
        const transcriptionUnlisten = await listen('transcription-update', (event) => {
          console.log('🎤 [useVoiceInput] Received transcription-update event:', event.payload);
          const { text, is_final, segment_id, confidence } = event.payload;
          
          if (is_final) {
            console.log('🎤 [useVoiceInput] Processing final transcription:', text);
            setTranscription(prev => {
              // Replace or append the final transcription
              const lines = prev.split('\n').filter(line => line.trim());
              lines.push(text);
              return lines.join('\n');
            });
          } else {
            console.log('🎤 [useVoiceInput] Processing partial transcription:', text);
            // Show partial transcription
            setTranscription(prev => {
              const lines = prev.split('\n').filter(line => line.trim());
              return [...lines, `[${text}]`].join('\n');
            });
          }
        });

        // Listen for transcription completion
        console.log('🎤 [useVoiceInput] Setting up transcription-complete listener...');
        const completionUnlisten = await listen('transcription-complete', (event) => {
          console.log('🎤 [useVoiceInput] Received transcription-complete event:', event.payload);
          const { final_text, session_id, segment_count } = event.payload;
          setTranscription(final_text);
          setSessionInfo(prev => ({
            ...prev,
            segment_count
          }));
        });

        // Listen for errors
        console.log('🎤 [useVoiceInput] Setting up transcription-error listener...');
        const errorUnlisten = await listen('transcription-error', (event) => {
          console.error('🎤 [useVoiceInput] Received transcription-error event:', event.payload);
          setError(event.payload.message || 'Transcription error occurred');
        });

        // Listen for status updates
        console.log('🎤 [useVoiceInput] Setting up transcription-status listener...');
        const statusUnlisten = await listen('transcription-status', (event) => {
          console.log('🎤 [useVoiceInput] Received transcription-status event:', event.payload);
          setStatus(event.payload);
          setIsRecording(event.payload.recording);
          setIsReady(event.payload.ready);
        });

        eventListenersRef.current = [
          transcriptionUnlisten,
          completionUnlisten,
          errorUnlisten,
          statusUnlisten
        ];
        
        console.log('🎤 [useVoiceInput] All event listeners set up successfully!');
      } catch (err) {
        console.error('🎤 [useVoiceInput] Failed to set up event listeners:', err);
        setError(err.toString());
      }
    };

    setupEventListeners();

    return () => {
      console.log('🎤 [useVoiceInput] Cleaning up event listeners...');
      // Clean up event listeners
      eventListenersRef.current.forEach(unlisten => {
        if (typeof unlisten === 'function') {
          unlisten();
        }
      });
      eventListenersRef.current = [];
      console.log('🎤 [useVoiceInput] Event listeners cleaned up');
    };
  }, []);

  // Initialize on mount
  useEffect(() => {
    initialize();
  }, [initialize]);

  // Start recording
  const startRecording = useCallback(async () => {
    try {
      setError(null);
      
      if (!isInitialized || !isReady) {
        throw new Error('Transcription service not ready. Please wait for initialization.');
      }

      const result = await invoke('start_recording');
      console.log('Recording started:', result);
      setIsRecording(true);
      
      // Clear previous transcription
      setTranscription('');
      
      // Update status
      const currentStatus = await invoke('get_status');
      setStatus(currentStatus);
      
    } catch (err) {
      console.error('Failed to start recording:', err);
      setError(err.toString());
    }
  }, [isInitialized, isReady]);

  // Stop recording
  const stopRecording = useCallback(async () => {
    try {
      setError(null);
      const result = await invoke('stop_recording');
      console.log('Recording stopped:', result);
      setIsRecording(false);
      
      // Update status and stats
      const currentStatus = await invoke('get_status');
      setStatus(currentStatus);
      
      const currentStats = await invoke('get_transcription_stats');
      setStats(currentStats);
      
    } catch (err) {
      console.error('Failed to stop recording:', err);
      setError(err.toString());
    }
  }, []);

  // Toggle recording
  const toggleRecording = useCallback(async () => {
    if (isRecording) {
      await stopRecording();
    } else {
      await startRecording();
    }
  }, [isRecording, startRecording, stopRecording]);

  // Clear transcription
  const clearTranscription = useCallback(() => {
    setTranscription('');
    setError(null);
  }, []);

  // Update configuration
  const updateConfig = useCallback(async (config) => {
    try {
      setError(null);
      const result = await invoke('update_transcription_config', { config });
      console.log('Config updated:', result);
      
      // Refresh status
      const currentStatus = await invoke('get_status');
      setStatus(currentStatus);
      
    } catch (err) {
      console.error('Failed to update config:', err);
      setError(err.toString());
    }
  }, []);

  // Refresh status
  const refreshStatus = useCallback(async () => {
    try {
      setError(null);
      
      const [currentStatus, sessionData, modelData, currentStats] = await Promise.all([
        invoke('get_status'),
        invoke('get_session_info'),
        invoke('get_model_status'),
        invoke('get_transcription_stats')
      ]);
      
      setStatus(currentStatus);
      setSessionInfo(sessionData);
      setModelStatus(modelData);
      setStats(currentStats);
      setIsRecording(currentStatus.recording);
      setIsReady(currentStatus.ready);
      setIsInitialized(currentStatus.initialized);
      
    } catch (err) {
      console.error('Failed to refresh status:', err);
      setError(err.toString());
    }
  }, []);

  // Test transcription
  const testTranscription = useCallback(async () => {
    try {
      setError(null);
      const result = await invoke('test_transcription');
      console.log('Test result:', result);
      return result;
    } catch (err) {
      console.error('Test failed:', err);
      setError(err.toString());
      throw err;
    }
  }, []);

  // Clear model cache
  const clearCache = useCallback(async () => {
    try {
      setError(null);
      const result = await invoke('clear_model_cache');
      console.log('Cache cleared:', result);
      
      // Refresh status after clearing cache
      await refreshStatus();
      
      return result;
    } catch (err) {
      console.error('Failed to clear cache:', err);
      setError(err.toString());
      throw err;
    }
  }, [refreshStatus]);

  return {
    // State - mapped to match App.jsx expectations
    isListening: isRecording,
    transcript: transcription,
    interimTranscript: '', // Not implemented in this version
    isSupported: isInitialized, // Show button when initialized
    error,
    isInitializing: !isInitialized && !error, // Show as initializing until ready
    isProcessing: false, // Not implemented in this version
    fullTranscript: transcription,
    
    // Original state (for backward compatibility)
    isRecording,
    isReady,
    isInitialized,
    transcription,
    status,
    sessionInfo,
    modelStatus,
    stats,
    
    // Actions - mapped to match App.jsx expectations
    startListening: startRecording,
    stopListening: stopRecording,
    resetTranscript: clearTranscription,
    
    // Original actions (for backward compatibility)
    startRecording,
    stopRecording,
    toggleRecording,
    clearTranscription,
    updateConfig,
    refreshStatus,
    testTranscription,
    clearCache,
    initialize,
  };
};

export default useVoiceInput;

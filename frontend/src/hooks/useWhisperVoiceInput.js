import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const useWhisperVoiceInput = () => {
  const [isListening, setIsListening] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [interimTranscript, setInterimTranscript] = useState('');
  const [isSupported, setIsSupported] = useState(false);
  const [error, setError] = useState(null);
  const [isInitializing, setIsInitializing] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  
  const unlistenRefs = useRef([]);
  const initializationPromise = useRef(null);

  // Initialize Whisper on component mount
  useEffect(() => {
    const initializeWhisper = async () => {
      if (initializationPromise.current) {
        return initializationPromise.current;
      }

      setIsInitializing(true);
      setError(null);

      initializationPromise.current = (async () => {
        try {
          const result = await invoke('initialize_whisper');
          console.log('Whisper initialized:', result);
          setIsSupported(true);
          setError(null);
          return true;
        } catch (err) {
          console.error('Failed to initialize Whisper:', err);
          setError(`Failed to initialize Whisper: ${err}`);
          setIsSupported(false);
          return false;
        } finally {
          setIsInitializing(false);
        }
      })();

      return initializationPromise.current;
    };

    initializeWhisper();
  }, []);

  // Set up event listeners
  useEffect(() => {
    const setupListeners = async () => {
      try {
        // Listen for partial transcription updates
        const unlistenPartial = await listen('transcription_partial', (event) => {
          console.log('🎤 Partial transcription received:', event.payload);
          console.log('🎤 Current interim transcript:', interimTranscript);
          setInterimTranscript(event.payload);
          setIsProcessing(true);
        });

        // Listen for final transcription results
        const unlistenFinal = await listen('transcription_final', (event) => {
          console.log('🎤 Final transcription received:', event.payload);
          console.log('🎤 Current transcript before update:', transcript);
          setTranscript(prev => {
            const newText = prev + (prev ? ' ' : '') + event.payload;
            console.log('🎤 Updated transcript:', newText);
            return newText;
          });
          setInterimTranscript('');
          setIsProcessing(false);
        });

        // Listen for recording started
        const unlistenStarted = await listen('recording_started', () => {
          console.log('Recording started');
          setIsListening(true);
          setError(null);
        });

        // Listen for recording stopped
        const unlistenStopped = await listen('recording_stopped', () => {
          console.log('Recording stopped');
          setIsListening(false);
          setIsProcessing(false);
        });

        // Listen for transcription errors
        const unlistenError = await listen('transcription_error', (event) => {
          console.error('Transcription error:', event.payload);
          setError(event.payload);
          setIsListening(false);
          setIsProcessing(false);
        });

        // Store unlisten functions
        unlistenRefs.current = [
          unlistenPartial,
          unlistenFinal,
          unlistenStarted,
          unlistenStopped,
          unlistenError
        ];

      } catch (err) {
        console.error('Failed to set up event listeners:', err);
        setError(`Failed to set up event listeners: ${err}`);
      }
    };

    setupListeners();

    // Cleanup function
    return () => {
      unlistenRefs.current.forEach(unlisten => {
        if (typeof unlisten === 'function') {
          unlisten();
        }
      });
      unlistenRefs.current = [];
    };
  }, []);

  const startListening = useCallback(async () => {
    if (!isSupported) {
      setError('Whisper is not initialized');
      return;
    }

    if (isListening) {
      console.log('Already listening');
      return;
    }

    try {
      setError(null);
      setTranscript('');
      setInterimTranscript('');
      
      const result = await invoke('start_realtime_recording');
      console.log('Started recording:', result);
    } catch (err) {
      console.error('Failed to start recording:', err);
      setError(`Failed to start recording: ${err}`);
      setIsListening(false);
    }
  }, [isSupported, isListening]);

  const stopListening = useCallback(async () => {
    if (!isListening) {
      return;
    }

    try {
      const result = await invoke('stop_recording');
      console.log('Stopped recording:', result);
    } catch (err) {
      console.error('Failed to stop recording:', err);
      setError(`Failed to stop recording: ${err}`);
    }
  }, [isListening]);

  const resetTranscript = useCallback(() => {
    setTranscript('');
    setInterimTranscript('');
  }, []);

  // Check if Whisper is ready
  const checkWhisperStatus = useCallback(async () => {
    try {
      const status = await invoke('get_whisper_status');
      return status;
    } catch (err) {
      console.error('Failed to get Whisper status:', err);
      return { initialized: false, recording: false, ready: false };
    }
  }, []);

  return {
    isListening,
    transcript,
    interimTranscript,
    isSupported,
    error,
    isInitializing,
    isProcessing,
    startListening,
    stopListening,
    resetTranscript,
    checkWhisperStatus,
    // Combined transcript for display (same interface as original hook)
    fullTranscript: transcript + (interimTranscript ? ` ${interimTranscript}` : '')
  };
};

export default useWhisperVoiceInput;

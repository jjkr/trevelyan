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
  
  const unlistenFunctions = useRef([]);

  // Initialize Whisper and set up event listeners
  useEffect(() => {
    let isMounted = true;
    const unlistenFuncs = [];

    const setup = async () => {
      setIsInitializing(true);
      setError(null);

      try {
        const isInitialized = await invoke('ensure_whisper_initialized');
        if (!isMounted) return;

        if (!isInitialized) {
          // Wait for initialization
          // This could be improved with a subscription to an initialization event
          await new Promise(resolve => setTimeout(resolve, 2000));
        }

        console.log('Whisper is ready');
        setIsSupported(true);

        // Set up listeners after successful initialization
        const unlistenPartial = await listen('transcription_partial', (event) => {
          setInterimTranscript(event.payload);
          setIsProcessing(true);
        });
        unlistenFuncs.push(unlistenPartial);

        const unlistenFinal = await listen('transcription_final', (event) => {
          setTranscript(event.payload);
          setInterimTranscript('');
          setIsProcessing(false);
        });
        unlistenFuncs.push(unlistenFinal);

        const unlistenStarted = await listen('recording_started', () => {
          setIsListening(true);
          setError(null);
        });
        unlistenFuncs.push(unlistenStarted);

        const unlistenStopped = await listen('recording_stopped', () => {
          setIsListening(false);
          setIsProcessing(false);
        });
        unlistenFuncs.push(unlistenStopped);

        const unlistenError = await listen('transcription_error', (event) => {
          console.error('Transcription error:', event.payload);
          setError(event.payload);
          setIsListening(false);
          setIsProcessing(false);
        });
        unlistenFuncs.push(unlistenError);
        
        unlistenFunctions.current = unlistenFuncs;

      } catch (err) {
        if (isMounted) {
          console.error('Failed to setup Whisper:', err);
          setError(`Failed to setup Whisper: ${err}`);
          setIsSupported(false);
        }
      } finally {
        if (isMounted) {
          setIsInitializing(false);
        }
      }
    };

    setup();

    // Cleanup function
    return () => {
      isMounted = false;
      console.log('Cleaning up Whisper listeners');
      unlistenFunctions.current.forEach(unlisten => unlisten());
      unlistenFunctions.current = [];
    };
  }, []); // Empty dependency array ensures this runs only once on mount

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

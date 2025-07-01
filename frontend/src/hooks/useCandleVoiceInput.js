import { useState, useEffect, useRef, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const useCandleVoiceInput = () => {
  const [isListening, setIsListening] = useState(false);
  const [segments, setSegments] = useState([]);
  const [displayText, setDisplayText] = useState('');
  const [isSupported, setIsSupported] = useState(false);
  const [error, setError] = useState(null);
  const [isInitializing, setIsInitializing] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [sessionInfo, setSessionInfo] = useState(null);
  const [stats, setStats] = useState(null);
  
  const unlistenFunctions = useRef([]);

  // Initialize Candle transcription and set up event listeners
  useEffect(() => {
    let isMounted = true;
    const unlistenFuncs = [];

    const setup = async () => {
      setIsInitializing(true);
      setError(null);

      try {
        const isInitialized = await invoke('ensure_candle_initialized');
        if (!isMounted) return;

        if (!isInitialized) {
          // Wait for initialization with polling
          let attempts = 0;
          const maxAttempts = 30; // 30 seconds max wait
          
          while (attempts < maxAttempts && !await invoke('ensure_candle_initialized')) {
            await new Promise(resolve => setTimeout(resolve, 1000));
            attempts++;
            if (!isMounted) return;
          }
        }

        console.log('Candle transcription is ready');
        setIsSupported(true);

        // Set up listeners for the new enhanced events
        const unlistenStream = await listen('transcription_stream', (event) => {
          const { segments: newSegments, session_id, total_duration } = event.payload;
          
          setSegments(prevSegments => {
            const merged = mergeSegments(prevSegments, newSegments);
            const display = generateDisplayText(merged);
            setDisplayText(display);
            return merged;
          });
          
          setIsProcessing(true);
        });
        unlistenFuncs.push(unlistenStream);

        const unlistenUpdate = await listen('transcription_update', (event) => {
          const { segment_id, new_text, confidence, status } = event.payload;
          
          setSegments(prev => prev.map(segment => 
            segment.id === segment_id 
              ? { ...segment, text: new_text, confidence, status }
              : segment
          ));
        });
        unlistenFuncs.push(unlistenUpdate);

        const unlistenComplete = await listen('transcription_complete', (event) => {
          setDisplayText(event.payload);
          setIsProcessing(false);
          
          // Update final stats
          updateSessionInfo();
        });
        unlistenFuncs.push(unlistenComplete);

        // Backward compatibility events
        const unlistenPartial = await listen('transcription_partial', (event) => {
          // This is handled by transcription_stream now, but kept for compatibility
          setIsProcessing(true);
        });
        unlistenFuncs.push(unlistenPartial);

        const unlistenFinal = await listen('transcription_final', (event) => {
          // This is handled by transcription_complete now, but kept for compatibility
          setIsProcessing(false);
        });
        unlistenFuncs.push(unlistenFinal);

        const unlistenStarted = await listen('recording_started', () => {
          setIsListening(true);
          setError(null);
          setSegments([]);
          setDisplayText('');
        });
        unlistenFuncs.push(unlistenStarted);

        const unlistenStopped = await listen('recording_stopped', () => {
          setIsListening(false);
          setIsProcessing(false);
          updateSessionInfo();
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
          console.error('Failed to setup Candle transcription:', err);
          setError(`Failed to setup Candle transcription: ${err}`);
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
      console.log('Cleaning up Candle transcription listeners');
      unlistenFunctions.current.forEach(unlisten => unlisten());
      unlistenFunctions.current = [];
    };
  }, []);

  // Helper function to merge new segments with existing ones
  const mergeSegments = useCallback((existingSegments, newSegments) => {
    const merged = [...existingSegments];
    
    newSegments.forEach(newSegment => {
      const existingIndex = merged.findIndex(seg => seg.id === newSegment.id);
      if (existingIndex >= 0) {
        // Update existing segment
        merged[existingIndex] = newSegment;
      } else {
        // Add new segment in chronological order
        const insertIndex = merged.findIndex(seg => seg.start_time > newSegment.start_time);
        if (insertIndex >= 0) {
          merged.splice(insertIndex, 0, newSegment);
        } else {
          merged.push(newSegment);
        }
      }
    });
    
    return merged;
  }, []);

  // Helper function to generate display text with confidence styling
  const generateDisplayText = useCallback((segments) => {
    return segments
      .sort((a, b) => a.start_time - b.start_time)
      .map(segment => segment.text)
      .join(' ');
  }, []);

  // Update session information
  const updateSessionInfo = useCallback(async () => {
    try {
      const info = await invoke('get_session_info');
      setSessionInfo(info);
      
      const statistics = await invoke('get_transcription_stats');
      setStats(statistics);
    } catch (err) {
      console.error('Failed to get session info:', err);
    }
  }, []);

  const startListening = useCallback(async () => {
    if (!isSupported) {
      setError('Candle transcription is not initialized');
      return;
    }

    if (isListening) {
      console.log('Already listening');
      return;
    }

    try {
      setError(null);
      setSegments([]);
      setDisplayText('');
      
      const result = await invoke('start_candle_recording');
      console.log('Started Candle recording:', result);
    } catch (err) {
      console.error('Failed to start Candle recording:', err);
      setError(`Failed to start recording: ${err}`);
      setIsListening(false);
    }
  }, [isSupported, isListening]);

  const stopListening = useCallback(async () => {
    if (!isListening) {
      return;
    }

    try {
      const result = await invoke('stop_candle_recording');
      console.log('Stopped Candle recording:', result);
    } catch (err) {
      console.error('Failed to stop Candle recording:', err);
      setError(`Failed to stop recording: ${err}`);
    }
  }, [isListening]);

  const resetTranscript = useCallback(() => {
    setSegments([]);
    setDisplayText('');
    setStats(null);
  }, []);

  // Check transcription status
  const checkStatus = useCallback(async () => {
    try {
      const status = await invoke('get_candle_status');
      return status;
    } catch (err) {
      console.error('Failed to get Candle status:', err);
      return { initialized: false, recording: false, ready: false };
    }
  }, []);

  // Update configuration
  const updateConfig = useCallback(async (config) => {
    try {
      await invoke('update_transcription_config', { config });
      console.log('Configuration updated:', config);
    } catch (err) {
      console.error('Failed to update config:', err);
      setError(`Failed to update config: ${err}`);
    }
  }, []);

  // Test transcription
  const testTranscription = useCallback(async () => {
    try {
      const result = await invoke('test_transcription');
      console.log('Test transcription result:', result);
      return result;
    } catch (err) {
      console.error('Test transcription failed:', err);
      throw err;
    }
  }, []);

  // Get segments with confidence styling information
  const getStyledSegments = useCallback(() => {
    return segments.map(segment => ({
      ...segment,
      className: getConfidenceClass(segment.status, segment.confidence),
      isProvisional: segment.status === 'provisional',
      isStable: segment.status === 'stable',
      isFinal: segment.status === 'final',
    }));
  }, [segments]);

  // Helper to get CSS class based on confidence and status
  const getConfidenceClass = (status, confidence) => {
    switch (status) {
      case 'provisional':
        return 'transcription-provisional';
      case 'stable':
        return 'transcription-stable';
      case 'final':
        return 'transcription-final';
      default:
        return 'transcription-unknown';
    }
  };

  return {
    // Basic interface (compatible with original hook)
    isListening,
    transcript: displayText,
    interimTranscript: '', // Not used in new system, kept for compatibility
    isSupported,
    error,
    isInitializing,
    isProcessing,
    startListening,
    stopListening,
    resetTranscript,
    
    // Enhanced interface
    segments,
    displayText,
    sessionInfo,
    stats,
    checkStatus,
    updateConfig,
    testTranscription,
    getStyledSegments,
    
    // Combined transcript for display (same interface as original hook)
    fullTranscript: displayText,
    
    // Utility functions
    mergeSegments,
    generateDisplayText,
    updateSessionInfo,
  };
};

export default useCandleVoiceInput;

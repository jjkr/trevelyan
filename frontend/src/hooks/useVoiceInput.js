import { useState, useEffect, useRef, useCallback } from 'react';

const useVoiceInput = () => {
  const [isListening, setIsListening] = useState(false);
  const [transcript, setTranscript] = useState('');
  const [interimTranscript, setInterimTranscript] = useState('');
  const [isSupported, setIsSupported] = useState(false);
  const [error, setError] = useState(null);
  
  const recognitionRef = useRef(null);
  const timeoutRef = useRef(null);

  // Check browser support and initialize recognition
  useEffect(() => {
    const SpeechRecognition = window.SpeechRecognition || window.webkitSpeechRecognition;
    if (!SpeechRecognition) {
      setIsSupported(false);
      setError('Speech recognition is not supported in this browser');
      return;
    }

    setIsSupported(true);
    const recognition = new SpeechRecognition();
    recognitionRef.current = recognition;

    // Configure recognition
    recognition.continuous = true;
    recognition.interimResults = true;
    recognition.lang = 'en-US';
    recognition.maxAlternatives = 1;

    // Handle results
    recognition.onresult = (event) => {
      let finalTranscript = '';
      let interim = '';

      for (let i = 0; i < event.results.length; i++) {
        const transcriptPart = event.results[i][0].transcript;
        if (event.results[i].isFinal) {
          finalTranscript += transcriptPart;
        } else {
          interim += transcriptPart;
        }
      }

      if (finalTranscript) {
        setTranscript(finalTranscript);
      }
      setInterimTranscript(interim);

      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      timeoutRef.current = setTimeout(() => {
        recognition.stop();
      }, 5000);
    };

    // Handle errors
    recognition.onerror = (event) => {
      console.error('Speech recognition error:', event.error);
      if (event.error !== 'aborted') {
        setError(`Speech recognition error: ${event.error}`);
      }
      setIsListening(false);
    };

    // Handle end
    recognition.onend = () => {
      setIsListening(false);
      setInterimTranscript('');
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
    };

    // Handle start
    recognition.onstart = () => {
      setError(null);
      setIsListening(true);
    };

    // Cleanup
    return () => {
      console.log('Cleaning up speech recognition');
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      recognition.abort();
    };
  }, []); // Empty dependency array ensures this runs only once on mount

  const startListening = useCallback(() => {
    if (!isSupported) {
      setError('Speech recognition is not supported');
      return;
    }

    if (recognitionRef.current && !isListening) {
      // Stop any existing recognition first
      try {
        recognitionRef.current.stop();
      } catch (e) {
        // Ignore errors when stopping
      }

      // Clear any existing timeout
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }

      setTranscript('');
      setInterimTranscript('');
      setError(null);
      
      // Small delay to ensure previous recognition is fully stopped
      setTimeout(() => {
        try {
          if (recognitionRef.current) {
            recognitionRef.current.start();
          }
        } catch (error) {
          console.error('Error starting speech recognition:', error);
          if (error.name === 'InvalidStateError') {
            setError('Speech recognition is already running. Please try again.');
          } else {
            setError('Failed to start speech recognition');
          }
        }
      }, 100);
    }
  }, [isSupported, isListening]);

  const stopListening = useCallback(() => {
    if (recognitionRef.current) {
      try {
        recognitionRef.current.stop();
      } catch (e) {
        // Ignore errors when stopping
      }
    }
    if (timeoutRef.current) {
      clearTimeout(timeoutRef.current);
    }
  }, []);

  const resetTranscript = useCallback(() => {
    setTranscript('');
    setInterimTranscript('');
  }, []);

  return {
    isListening,
    transcript,
    interimTranscript,
    isSupported,
    error,
    startListening,
    stopListening,
    resetTranscript,
    // Combined transcript for display
    fullTranscript: transcript + interimTranscript
  };
};

export default useVoiceInput;

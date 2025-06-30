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
    if (SpeechRecognition) {
      setIsSupported(true);
      recognitionRef.current = new SpeechRecognition();
      
      // Configure recognition
      const recognition = recognitionRef.current;
      recognition.continuous = true;
      recognition.interimResults = true;
      recognition.lang = 'en-US';
      recognition.maxAlternatives = 1;

      // Handle results
      recognition.onresult = (event) => {
        let finalTranscript = '';
        let interimTranscript = '';

        // Process all results from the beginning to get complete transcript
        for (let i = 0; i < event.results.length; i++) {
          const transcript = event.results[i][0].transcript;
          if (event.results[i].isFinal) {
            finalTranscript += transcript;
          } else {
            interimTranscript += transcript;
          }
        }

        // Set the complete final transcript (not accumulated)
        if (finalTranscript) {
          setTranscript(finalTranscript);
        }
        setInterimTranscript(interimTranscript);

        // Clear timeout and set new one for auto-stop
        if (timeoutRef.current) {
          clearTimeout(timeoutRef.current);
        }
        
        // Auto-stop after 5 seconds of silence
        timeoutRef.current = setTimeout(() => {
          if (recognitionRef.current) {
            try {
              recognitionRef.current.stop();
            } catch (e) {
              console.log('Recognition already stopped');
            }
          }
        }, 5000);
      };

      // Handle errors
      recognition.onerror = (event) => {
        console.error('Speech recognition error:', event.error);
        
        // Don't show error for aborted - it's usually intentional
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
    } else {
      setIsSupported(false);
      setError('Speech recognition is not supported in this browser');
    }

    // Cleanup
    return () => {
      if (timeoutRef.current) {
        clearTimeout(timeoutRef.current);
      }
      if (recognitionRef.current) {
        try {
          recognitionRef.current.abort();
        } catch (e) {
          // Ignore errors during cleanup
        }
      }
    };
  }, []); // Remove isListening dependency to prevent re-initialization

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

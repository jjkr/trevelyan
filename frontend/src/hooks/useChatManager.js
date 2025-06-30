import { useState, useCallback, useRef, useEffect } from 'react';
import LLMProviderFactory from '../services/LLMProviderFactory.js';
import {
  createUserMessage,
  createAIMessage,
  updateMessageText,
  completeMessage,
  failMessage,
  formatMessagesForAPI,
  getRecentMessages,
  truncateConversation
} from '../utils/messageUtils.js';

/**
 * Custom hook for managing chat state and streaming
 */
const useChatManager = (providerName = 'openrouter', options = {}) => {
  const [messages, setMessages] = useState([]);
  const [isStreaming, setIsStreaming] = useState(false);
  const [error, setError] = useState(null);
  const [currentProvider, setCurrentProvider] = useState(providerName);
  
  // Refs for cleanup and abort control
  const abortControllerRef = useRef(null);
  const streamingMessageIdRef = useRef(null);

  // Provider instance
  const [provider, setProvider] = useState(() => {
    try {
      return LLMProviderFactory.get(providerName);
    } catch (err) {
      console.error('Failed to initialize provider:', err);
      return null;
    }
  });

  // Update provider when providerName changes
  useEffect(() => {
    try {
      const newProvider = LLMProviderFactory.get(providerName);
      setProvider(newProvider);
      setCurrentProvider(providerName);
      setError(null);
    } catch (err) {
      console.error('Failed to switch provider:', err);
      setError(`Failed to switch to provider: ${providerName}`);
    }
  }, [providerName]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  }, []);

  /**
   * Send a message and handle streaming response
   */
  const sendMessage = useCallback(async (text, messageOptions = {}) => {
    console.log('sendMessage called with text:', text);
    if (!text.trim() || isStreaming || !provider) {
      console.log('sendMessage aborted:', { hasText: !!text.trim(), isStreaming, hasProvider: !!provider });
      return;
    }

    // Clear any previous errors
    setError(null);
    setIsStreaming(true);

    // Create abort controller for this request
    abortControllerRef.current = new AbortController();

    try {
      // Add user message
      const userMessage = createUserMessage(text);
      setMessages(prev => [...prev, userMessage]);

      // Create AI message placeholder
      const aiMessage = createAIMessage();
      streamingMessageIdRef.current = aiMessage.id;
      setMessages(prev => [...prev, aiMessage]);

      // Prepare messages for API
      const conversationHistory = [...messages, userMessage];
      const recentMessages = getRecentMessages(conversationHistory, options.maxContextMessages || 20);
      const truncatedMessages = truncateConversation(recentMessages, options.maxTokens || 3000);
      const apiMessages = formatMessagesForAPI(truncatedMessages);
      console.log('Messages sent to API:', apiMessages);

      // Validate provider configuration
      const validation = provider.validateConfig();
      if (!validation.isValid) {
        throw new Error(validation.errors.join(', '));
      }

      // Start streaming
      let accumulatedText = '';
      const stream = provider.streamChat(apiMessages, {
        model: options.model,
        temperature: options.temperature,
        maxTokens: options.maxResponseTokens || 2000,
        ...messageOptions
      });

      try {
        console.log('Starting to iterate over stream...');
        for await (const chunk of stream) {
          if (abortControllerRef.current?.signal.aborted) {
            console.log('Streaming aborted by user.');
            break;
          }
          console.log('Received chunk:', chunk);
          accumulatedText += chunk;
          setMessages(prev => prev.map(msg =>
            msg.id === streamingMessageIdRef.current
              ? updateMessageText(msg, accumulatedText)
              : msg
          ));
        }
        console.log('Finished iterating over stream.');
      } finally {
        // Capture the message ID before nullifying the ref to avoid race condition
        const messageId = streamingMessageIdRef.current;
        
        // Ensure message is marked as complete
        setMessages(prev => prev.map(msg =>
          msg.id === messageId
            ? completeMessage(updateMessageText(msg, accumulatedText))
            : msg
        ));
      }
    } catch (err) {
      console.error('Error sending message:', err);
      
      // Capture the message ID before nullifying the ref to avoid race condition
      const messageId = streamingMessageIdRef.current;
      
      // Update the streaming message with error
      if (messageId) {
        setMessages(prev => prev.map(msg => 
          msg.id === messageId
            ? failMessage(msg, err.message)
            : msg
        ));
      }
      
      setError(err.message);
    } finally {
      setIsStreaming(false);
      streamingMessageIdRef.current = null;
      abortControllerRef.current = null;
    }
  }, [messages, isStreaming, provider, options]);

  /**
   * Stop current streaming
   */
  const stopStreaming = useCallback(() => {
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }
    setIsStreaming(false);
  }, []);

  /**
   * Clear conversation
   */
  const clearConversation = useCallback(() => {
    if (isStreaming) {
      stopStreaming();
    }
    setMessages([]);
    setError(null);
  }, [isStreaming, stopStreaming]);

  /**
   * Retry last failed message
   */
  const retryLastMessage = useCallback(() => {
    const lastUserMessage = [...messages].reverse().find(msg => msg.sender === 'user');
    if (lastUserMessage && !isStreaming) {
      // Remove failed AI response if exists
      setMessages(prev => prev.filter(msg => 
        !(msg.sender === 'ai' && msg.status === 'failed')
      ));
      sendMessage(lastUserMessage.text);
    }
  }, [messages, isStreaming, sendMessage]);

  /**
   * Switch provider
   */
  const switchProvider = useCallback((newProviderName, config = {}) => {
    if (isStreaming) {
      stopStreaming();
    }
    
    try {
      const newProvider = LLMProviderFactory.create(newProviderName, config);
      setProvider(newProvider);
      setCurrentProvider(newProviderName);
      setError(null);
    } catch (err) {
      console.error('Failed to switch provider:', err);
      setError(`Failed to switch to provider: ${newProviderName}`);
    }
  }, [isStreaming, stopStreaming]);

  /**
   * Get provider information
   */
  const getProviderInfo = useCallback(() => {
    if (!provider) return null;
    
    return {
      ...provider.getMetadata(),
      validation: provider.validateConfig()
    };
  }, [provider]);

  return {
    // State
    messages,
    isStreaming,
    error,
    currentProvider,
    provider,
    
    // Actions
    sendMessage,
    stopStreaming,
    clearConversation,
    retryLastMessage,
    switchProvider,
    
    // Info
    getProviderInfo,
    
    // Computed
    hasMessages: messages.length > 0,
    lastMessage: messages[messages.length - 1] || null,
    canSend: !isStreaming && provider && provider.validateConfig().isValid
  };
};

export default useChatManager;

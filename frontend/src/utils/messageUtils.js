/**
 * Utility functions for message handling
 */

/**
 * Generate a unique message ID
 * @returns {string} - Unique message ID
 */
export const generateMessageId = () => {
  return `msg_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
};

/**
 * Create a user message object
 * @param {string} text - Message text
 * @returns {Object} - Message object
 */
export const createUserMessage = (text) => {
  return {
    id: generateMessageId(),
    text: text.trim(),
    sender: 'user',
    timestamp: new Date().toISOString(),
    status: 'sent'
  };
};

/**
 * Create an AI message object
 * @param {string} text - Message text (can be empty for streaming)
 * @returns {Object} - Message object
 */
export const createAIMessage = (text = '') => {
  return {
    id: generateMessageId(),
    text,
    sender: 'ai',
    timestamp: new Date().toISOString(),
    status: 'streaming',
    isStreaming: true
  };
};

/**
 * Update a message's text content
 * @param {Object} message - Message object
 * @param {string} newText - New text content
 * @returns {Object} - Updated message object
 */
export const updateMessageText = (message, newText) => {
  return {
    ...message,
    text: newText,
    timestamp: new Date().toISOString()
  };
};

/**
 * Mark a message as complete (stop streaming)
 * @param {Object} message - Message object
 * @returns {Object} - Updated message object
 */
export const completeMessage = (message) => {
  return {
    ...message,
    status: 'complete',
    isStreaming: false,
    completedAt: new Date().toISOString()
  };
};

/**
 * Mark a message as failed
 * @param {Object} message - Message object
 * @param {string} error - Error message
 * @returns {Object} - Updated message object
 */
export const failMessage = (message, error) => {
  return {
    ...message,
    status: 'failed',
    isStreaming: false,
    error,
    failedAt: new Date().toISOString()
  };
};

/**
 * Format messages for LLM API (remove internal fields)
 * @param {Array} messages - Array of message objects
 * @returns {Array} - Formatted messages for API
 */
export const formatMessagesForAPI = (messages) => {
  return messages
    .filter(msg => msg.status !== 'failed') // Exclude failed messages
    .map(msg => ({
      role: msg.sender === 'user' ? 'user' : 'assistant',
      content: msg.text
    }));
};

/**
 * Get the last N messages for context
 * @param {Array} messages - Array of message objects
 * @param {number} count - Number of messages to include
 * @returns {Array} - Last N messages
 */
export const getRecentMessages = (messages, count = 10) => {
  return messages.slice(-count);
};

/**
 * Calculate approximate token count for messages
 * @param {Array} messages - Array of message objects
 * @returns {number} - Approximate token count
 */
export const estimateTokenCount = (messages) => {
  const text = messages.map(msg => msg.text).join(' ');
  // Rough estimation: 1 token ≈ 4 characters
  return Math.ceil(text.length / 4);
};

/**
 * Truncate conversation to fit within token limit
 * @param {Array} messages - Array of message objects
 * @param {number} maxTokens - Maximum token count
 * @returns {Array} - Truncated messages
 */
export const truncateConversation = (messages, maxTokens = 4000) => {
  let truncated = [...messages];
  
  while (estimateTokenCount(truncated) > maxTokens && truncated.length > 1) {
    // Remove oldest messages but keep the first one if it's a system message
    if (truncated[0].sender === 'system') {
      truncated.splice(1, 1);
    } else {
      truncated.splice(0, 1);
    }
  }
  
  return truncated;
};

/**
 * Check if a message is currently streaming
 * @param {Object} message - Message object
 * @returns {boolean} - True if message is streaming
 */
export const isMessageStreaming = (message) => {
  return message.status === 'streaming' && message.isStreaming === true;
};

/**
 * Get conversation statistics
 * @param {Array} messages - Array of message objects
 * @returns {Object} - Conversation statistics
 */
export const getConversationStats = (messages) => {
  const userMessages = messages.filter(msg => msg.sender === 'user');
  const aiMessages = messages.filter(msg => msg.sender === 'ai');
  const failedMessages = messages.filter(msg => msg.status === 'failed');
  
  return {
    totalMessages: messages.length,
    userMessages: userMessages.length,
    aiMessages: aiMessages.length,
    failedMessages: failedMessages.length,
    estimatedTokens: estimateTokenCount(messages),
    lastActivity: messages.length > 0 ? messages[messages.length - 1].timestamp : null
  };
};

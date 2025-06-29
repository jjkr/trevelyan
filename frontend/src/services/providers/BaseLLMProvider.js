/**
 * Abstract base class for LLM providers
 * Defines the interface that all providers must implement
 */
class BaseLLMProvider {
  constructor(config = {}) {
    this.config = config;
    this.name = this.constructor.name.replace('Provider', '').toLowerCase();
  }

  /**
   * Stream chat completion
   * @param {Array} messages - Array of message objects
   * @param {Object} options - Provider-specific options
   * @returns {AsyncGenerator} - Yields content chunks
   */
  async *streamChat(messages, options = {}) {
    throw new Error(`streamChat method must be implemented by ${this.constructor.name}`);
  }

  /**
   * Send a complete message (non-streaming)
   * @param {Array} messages - Array of message objects
   * @param {Object} options - Provider-specific options
   * @returns {Promise<string>} - Complete response
   */
  async sendMessage(messages, options = {}) {
    const chunks = [];
    try {
      for await (const chunk of this.streamChat(messages, { ...options, stream: true })) {
        chunks.push(chunk);
      }
      return chunks.join('');
    } catch (error) {
      console.error(`Error in ${this.name} sendMessage:`, error);
      throw error;
    }
  }

  /**
   * Format messages for the provider's API
   * @param {Array} conversationHistory - Internal message format
   * @returns {Array} - Provider-specific message format
   */
  formatMessages(conversationHistory) {
    return conversationHistory.map(msg => ({
      role: msg.sender === 'user' ? 'user' : 'assistant',
      content: msg.text
    }));
  }

  /**
   * Get available models for this provider
   * @returns {Promise<Array>} - Array of model objects
   */
  async getModels() {
    throw new Error(`getModels method must be implemented by ${this.constructor.name}`);
  }

  /**
   * Validate provider configuration
   * @returns {Object} - { isValid: boolean, errors: Array }
   */
  validateConfig() {
    return { isValid: true, errors: [] };
  }

  /**
   * Get provider capabilities
   * @returns {Object} - Capabilities object
   */
  getCapabilities() {
    return {
      streaming: true,
      functionCalling: false,
      vision: false,
      multimodal: false
    };
  }

  /**
   * Get provider metadata
   * @returns {Object} - Provider metadata
   */
  getMetadata() {
    return {
      name: this.name,
      displayName: this.name.charAt(0).toUpperCase() + this.name.slice(1),
      description: `${this.name} LLM provider`,
      capabilities: this.getCapabilities()
    };
  }
}

export default BaseLLMProvider;

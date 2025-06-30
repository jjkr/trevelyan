import BaseLLMProvider from './BaseLLMProvider.js';

/**
 * OpenRouter LLM Provider
 * Supports multiple models through OpenRouter's API
 */
class OpenRouterProvider extends BaseLLMProvider {
  constructor(config = {}) {
    super(config);
    this.apiKey = config.apiKey || import.meta.env.VITE_OPENROUTER_API_KEY;
    this.baseUrl = config.baseUrl || 'https://openrouter.ai/api/v1';
    this.defaultModel = config.defaultModel || 'anthropic/claude-3.5-sonnet';
    this.appName = config.appName || 'Trevelyan Chat App';
  }

  /**
   * Validate OpenRouter configuration
   */
  validateConfig() {
    const errors = [];
    
    if (!this.apiKey) {
      errors.push('OpenRouter API key not found. Please set VITE_OPENROUTER_API_KEY in your .env file.');
    }
    
    if (!this.baseUrl) {
      errors.push('OpenRouter base URL is required.');
    }

    return {
      isValid: errors.length === 0,
      errors
    };
  }

  /**
   * Stream chat completion from OpenRouter
   */
  async *streamChat(messages, options = {}) {
    const validation = this.validateConfig();
    if (!validation.isValid) {
      throw new Error(validation.errors.join(', '));
    }

    const {
      model = this.defaultModel,
      temperature = 0.7,
      maxTokens = 4000,
      stream = true
    } = options;

    const requestBody = {
      model,
      messages,
      temperature,
      max_tokens: maxTokens,
      stream
    };
    console.log('OpenRouter Request Body:', JSON.stringify(requestBody, null, 2));

    try {
      console.log('Attempting to fetch from OpenRouter...');
      const response = await fetch(`${this.baseUrl}/chat/completions`, {
        method: 'POST',
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
          'Content-Type': 'application/json',
          'HTTP-Referer': window.location.origin,
          'X-Title': this.appName
        },
        body: JSON.stringify(requestBody)
      });
      console.log('OpenRouter Response Status:', response.status);

      if (!response.ok) {
        const errorData = await response.json().catch(() => ({}));
        console.error('OpenRouter API Error Data:', errorData);
        throw new Error(`OpenRouter API error: ${response.status} - ${errorData.error?.message || response.statusText}`);
      }

      if (!stream) {
        const data = await response.json();
        yield data.choices[0]?.message?.content || '';
        return;
      }

      // Handle streaming response
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = '';

      try {
        while (true) {
          const { done, value } = await reader.read();
          
          if (done) {
            console.log('Stream finished.');
            break;
          }

          const chunk = decoder.decode(value, { stream: true });
          console.log('Raw chunk received:', chunk);
          buffer += chunk;
          const lines = buffer.split('\n');
          buffer = lines.pop() || ''; // Keep incomplete line in buffer

          for (const line of lines) {
            const trimmedLine = line.trim();
            
            if (trimmedLine === '') continue;
            if (trimmedLine === 'data: [DONE]') {
              console.log('Received [DONE] signal.');
              return;
            }
            if (!trimmedLine.startsWith('data: ')) {
              console.warn('Skipping line without "data: " prefix:', trimmedLine);
              continue;
            }

            try {
              const jsonStr = trimmedLine.slice(6); // Remove 'data: ' prefix
              const data = JSON.parse(jsonStr);
              console.log('Parsed data:', data);
              
              const content = data.choices?.[0]?.delta?.content;
              if (content) {
                console.log('Yielding content:', content);
                yield content;
              } else {
                console.log('No content in this chunk.');
              }
            } catch (parseError) {
              console.warn('Failed to parse SSE data:', trimmedLine, parseError);
            }
          }
        }
      } finally {
        reader.releaseLock();
      }
    } catch (error) {
      console.error('OpenRouter streaming error:', error);
      throw error;
    }
  }

  /**
   * Get available models from OpenRouter
   */
  async getModels() {
    const validation = this.validateConfig();
    if (!validation.isValid) {
      throw new Error(validation.errors.join(', '));
    }

    try {
      const response = await fetch(`${this.baseUrl}/models`, {
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
        }
      });

      if (!response.ok) {
        throw new Error(`Failed to fetch models: ${response.statusText}`);
      }

      const data = await response.json();
      return data.data || [];
    } catch (error) {
      console.error('Error fetching OpenRouter models:', error);
      throw error;
    }
  }

  /**
   * Get OpenRouter capabilities
   */
  getCapabilities() {
    return {
      streaming: true,
      functionCalling: true,
      vision: true,
      multimodal: true,
      modelSelection: true
    };
  }

  /**
   * Get OpenRouter metadata
   */
  getMetadata() {
    return {
      name: 'openrouter',
      displayName: 'OpenRouter',
      description: 'Access to multiple LLM providers through OpenRouter',
      capabilities: this.getCapabilities(),
      website: 'https://openrouter.ai',
      defaultModel: this.defaultModel
    };
  }

  /**
   * Get popular models for quick selection
   */
  getPopularModels() {
    return [
      {
        id: 'anthropic/claude-3.5-sonnet',
        name: 'Claude 3.5 Sonnet',
        provider: 'Anthropic',
        description: 'Most capable model for complex tasks'
      },
      {
        id: 'openai/gpt-4o',
        name: 'GPT-4o',
        provider: 'OpenAI',
        description: 'Latest GPT-4 with vision capabilities'
      },
      {
        id: 'anthropic/claude-3-haiku',
        name: 'Claude 3 Haiku',
        provider: 'Anthropic',
        description: 'Fast and efficient for simple tasks'
      },
      {
        id: 'openai/gpt-3.5-turbo',
        name: 'GPT-3.5 Turbo',
        provider: 'OpenAI',
        description: 'Cost-effective for most conversations'
      }
    ];
  }
}

export default OpenRouterProvider;

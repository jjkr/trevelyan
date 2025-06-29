import OpenRouterProvider from './providers/OpenRouterProvider.js';

/**
 * Factory for creating and managing LLM providers
 */
class LLMProviderFactory {
  static providers = new Map();
  static instances = new Map();

  /**
   * Register a provider class
   * @param {string} name - Provider name
   * @param {class} ProviderClass - Provider class
   */
  static register(name, ProviderClass) {
    this.providers.set(name.toLowerCase(), ProviderClass);
  }

  /**
   * Create or get a provider instance
   * @param {string} providerName - Name of the provider
   * @param {Object} config - Provider configuration
   * @returns {BaseLLMProvider} - Provider instance
   */
  static create(providerName, config = {}) {
    const name = providerName.toLowerCase();
    const instanceKey = `${name}_${JSON.stringify(config)}`;
    
    // Return existing instance if available
    if (this.instances.has(instanceKey)) {
      return this.instances.get(instanceKey);
    }

    const ProviderClass = this.providers.get(name);
    if (!ProviderClass) {
      throw new Error(`Provider '${providerName}' not found. Available providers: ${Array.from(this.providers.keys()).join(', ')}`);
    }

    const instance = new ProviderClass(config);
    this.instances.set(instanceKey, instance);
    return instance;
  }

  /**
   * Get a provider instance with default configuration
   * @param {string} providerName - Name of the provider
   * @returns {BaseLLMProvider} - Provider instance
   */
  static get(providerName) {
    return this.create(providerName);
  }

  /**
   * Get all available provider names
   * @returns {Array<string>} - Array of provider names
   */
  static getAvailable() {
    return Array.from(this.providers.keys());
  }

  /**
   * Get metadata for all available providers
   * @returns {Array<Object>} - Array of provider metadata
   */
  static getProviderMetadata() {
    const metadata = [];
    for (const [name, ProviderClass] of this.providers) {
      try {
        const instance = new ProviderClass();
        metadata.push(instance.getMetadata());
      } catch (error) {
        console.warn(`Failed to get metadata for provider ${name}:`, error);
      }
    }
    return metadata;
  }

  /**
   * Check if a provider is available
   * @param {string} providerName - Name of the provider
   * @returns {boolean} - True if provider is available
   */
  static isAvailable(providerName) {
    return this.providers.has(providerName.toLowerCase());
  }

  /**
   * Validate a provider's configuration
   * @param {string} providerName - Name of the provider
   * @param {Object} config - Provider configuration
   * @returns {Object} - Validation result
   */
  static validateProvider(providerName, config = {}) {
    try {
      const provider = this.create(providerName, config);
      return provider.validateConfig();
    } catch (error) {
      return {
        isValid: false,
        errors: [error.message]
      };
    }
  }

  /**
   * Clear all cached instances
   */
  static clearCache() {
    this.instances.clear();
  }

  /**
   * Get the default provider name
   * @returns {string} - Default provider name
   */
  static getDefaultProvider() {
    const envDefault = import.meta.env.VITE_DEFAULT_LLM_PROVIDER;
    if (envDefault && this.isAvailable(envDefault)) {
      return envDefault.toLowerCase();
    }
    
    // Fallback to first available provider
    const available = this.getAvailable();
    return available.length > 0 ? available[0] : null;
  }
}

// Register built-in providers
LLMProviderFactory.register('openrouter', OpenRouterProvider);

export default LLMProviderFactory;

import { BedrockRuntimeClient, InvokeModelWithResponseStreamCommand } from '@aws-sdk/client-bedrock-runtime';

class BedrockService {
  constructor() {
    // Configure AWS client with credentials
    const config = {
      region: import.meta.env.VITE_AWS_REGION || 'us-east-1',
    };

    // Only add credentials if they're provided via environment variables
    if (import.meta.env.VITE_AWS_ACCESS_KEY_ID && import.meta.env.VITE_AWS_SECRET_ACCESS_KEY) {
      config.credentials = {
        accessKeyId: import.meta.env.VITE_AWS_ACCESS_KEY_ID,
        secretAccessKey: import.meta.env.VITE_AWS_SECRET_ACCESS_KEY,
      };
    }
    // Otherwise, the SDK will try to use default credential chain (AWS CLI, IAM roles, etc.)

    this.client = new BedrockRuntimeClient(config);
    this.modelId = 'deepseek-llm-r1';
  }

  async streamMessage(message, onChunk, onError) {
    try {
      const payload = {
        anthropic_version: "bedrock-2023-05-31",
        max_tokens: 4000,
        messages: [
          {
            role: "user",
            content: message
          }
        ]
      };

      const command = new InvokeModelWithResponseStreamCommand({
        modelId: this.modelId,
        body: JSON.stringify(payload),
        contentType: 'application/json',
        accept: 'application/json'
      });

      const response = await this.client.send(command);
      
      if (response.body) {
        for await (const chunk of response.body) {
          if (chunk.chunk?.bytes) {
            const chunkData = JSON.parse(new TextDecoder().decode(chunk.chunk.bytes));
            
            if (chunkData.type === 'content_block_delta' && chunkData.delta?.text) {
              onChunk(chunkData.delta.text);
            }
            
            if (chunkData.type === 'message_stop') {
              break;
            }
          }
        }
      }
    } catch (error) {
      console.error('Bedrock API error:', error);
      onError(error.message || 'Failed to get response from AI');
    }
  }
}

export default new BedrockService();

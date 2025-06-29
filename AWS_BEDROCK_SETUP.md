# AWS Bedrock Integration Setup

Your Tauri chat app now has AWS Bedrock integration with Claude 3.5 Sonnet! Here's how to set it up:

## Prerequisites

1. **AWS Account** with Bedrock access
2. **Claude 3.5 Sonnet model** enabled in your AWS region
3. **AWS credentials** configured

## Setup Instructions

### 1. Configure AWS Credentials

Choose one of these options:

#### Option A: Environment Variables (Recommended for development)
1. Copy the example environment file:
   ```bash
   cd frontend
   cp .env.example .env
   ```

2. Edit `frontend/.env` and add your AWS credentials:
   ```
   VITE_AWS_REGION=us-east-1
   VITE_AWS_ACCESS_KEY_ID=your_actual_access_key
   VITE_AWS_SECRET_ACCESS_KEY=your_actual_secret_key
   ```

#### Option B: AWS CLI Profile (Alternative)
1. Install and configure AWS CLI:
   ```bash
   aws configure
   ```
2. Leave the environment variables empty in `.env` - the SDK will use your default AWS profile

### 2. Enable Bedrock Model Access

1. Go to AWS Console → Bedrock → Model Access
2. Request access to **Anthropic Claude 3.5 Sonnet**
3. Wait for approval (usually instant for most accounts)

### 3. Run the Application

```bash
# Install dependencies (if not already done)
cd frontend && npm install

# Start the development server
npm run tauri dev
```

## Features

- ✅ **Streaming responses** - Text appears as it's generated
- ✅ **Error handling** - Shows errors if API calls fail
- ✅ **Loading states** - Visual feedback during API calls
- ✅ **Input validation** - Prevents empty messages and multiple simultaneous requests

## Troubleshooting

### Common Issues

1. **"Access Denied" errors**
   - Ensure your AWS credentials have Bedrock permissions
   - Check that Claude 3.5 Sonnet is enabled in your region

2. **"Model not found" errors**
   - Verify the model is available in your AWS region
   - Try switching to `us-east-1` region

3. **Network/timeout errors**
   - Check your internet connection
   - Verify AWS service status

### Model Configuration

The app is configured to use:
- **Model**: `anthropic.claude-3-5-sonnet-20241022-v2:0`
- **Max tokens**: 4000
- **Region**: us-east-1 (default)

You can modify these settings in `frontend/src/services/bedrock.js`.

## Security Notes

- Never commit your `.env` file with real credentials
- Consider using IAM roles in production
- The `.env.example` file is safe to commit (contains no real credentials)

## Next Steps

Consider adding:
- Message history persistence
- Multiple model selection
- Custom system prompts
- Conversation management
- Token usage tracking

import React, { useState, useRef, useEffect } from 'react';
import './App.css';
import logo from './assets/OriginalLogo.png';
import useChatManager from './hooks/useChatManager.js';
import StreamingMessage from './components/StreamingMessage.jsx';

function App() {
  const [input, setInput] = useState('');
  const messagesEndRef = useRef(null);
  const textareaRef = useRef(null);

  // Initialize chat manager with OpenRouter
  const {
    messages,
    isStreaming,
    error,
    currentProvider,
    sendMessage,
    stopStreaming,
    clearConversation,
    retryLastMessage,
    canSend,
    getProviderInfo
  } = useChatManager('openrouter');

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages]);

  // Auto-resize textarea
  useEffect(() => {
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto';
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight-8}px`; // JJK - subtracting pixels for padding
    }
  }, [input]);

  const handleSendMessage = async (e) => {
    e.preventDefault();
    if (input.trim() && canSend) {
      const messageText = input;
      setInput('');
      await sendMessage(messageText);
      if (textareaRef.current) {
        textareaRef.current.style.height = 'auto'; // Reset height after sending
      }
    }
  };

  const handleKeyPress = (e) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage(e);
    }
  };

  const handleRetry = () => {
    retryLastMessage();
  };

  const providerInfo = getProviderInfo();

  return (
    <div className="app-container">
      {/* Header with provider info */}
      <div className="chat-header">
        <div className="provider-info">
          <img src={logo} alt="Trevelyan" className="header-logo" />
          <span className="provider-name">
            {providerInfo?.displayName || 'OpenRouter'}
          </span>
          {!providerInfo?.validation.isValid && (
            <span className="provider-error">
              ⚠️ Configuration needed
            </span>
          )}
        </div>
        {messages.length > 0 && (
          <button 
            className="clear-button"
            onClick={clearConversation}
            disabled={isStreaming}
          >
            Clear
          </button>
        )}
      </div>

      {/* Messages */}
      <div className="chat-messages">
        {messages.length === 0 && (
          <div className="welcome-message">
            <img src={logo} alt="Trevelyan Logo" className="welcome-logo" />
            <h2>Trevelyan</h2>
            <p>Your personal AI assistant.</p>
            {!providerInfo?.validation.isValid && (
              <div className="setup-notice">
                <p>⚠️ To get started, add your OpenRouter API key to the <code>.env</code> file.</p>
                <code>VITE_OPENROUTER_API_KEY=your_api_key_here</code>
              </div>
            )}
          </div>
        )}
        
        {messages.map((message) => (
          <StreamingMessage
            key={message.id}
            message={message}
            onRetry={message.status === 'failed' ? handleRetry : null}
          />
        ))}
        
        {/* Error display */}
        {error && (
          <div className="error-message">
            <div className="error-content">
              <strong>Error:</strong> {error}
            </div>
            <button onClick={() => window.location.reload()}>
              Reload App
            </button>
          </div>
        )}
        
        <div ref={messagesEndRef} />
      </div>

      {/* Input form */}
      <form onSubmit={handleSendMessage} className="chat-input-form">
        <div className="input-container">
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyPress={handleKeyPress}
            placeholder={
              canSend 
                ? "Ask away..." 
                : "Please configure your API key first..."
            }
            className="chat-input"
            disabled={!canSend}
            rows={1}
            style={{
              resize: 'none',
              maxHeight: '120px',
              overflow: 'auto'
            }}
          />
          
          {isStreaming ? (
            <button 
              type="button"
              onClick={stopStreaming}
              className="stop-button"
              aria-label="Stop generation"
            >
              ⏹️
            </button>
          ) : (
            <button 
              type="submit" 
              className="send-button"
              disabled={!input.trim() || !canSend}
              aria-label="Send message"
            >
              <img src={logo} alt="Send" className="send-button-logo" />
            </button>
          )}
        </div>
      </form>
    </div>
  );
}

export default App;

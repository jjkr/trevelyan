import React, { memo } from 'react';
import { isMessageStreaming } from '../utils/messageUtils.js';
import MarkdownRenderer from './MarkdownRenderer.jsx';

/**
 * Optimized component for rendering messages with streaming support
 */
const StreamingMessage = memo(({ message, onRetry }) => {
  const isStreaming = isMessageStreaming(message);
  const isCompleted = message.status === 'completed';
  const isFailed = message.status === 'failed';
  const isUser = message.sender === 'user';

  return (
    <div className={`message ${message.sender} ${message.status}`}>
      <div className="message-content">
        <MarkdownRenderer 
          content={message.text}
          isStreaming={isStreaming}
          isCompleted={isCompleted}
          sender={message.sender}
        />
      </div>
      
      {isFailed && (
        <div className="message-error">
          <div className="error-text">
            Failed to send message: {message.error}
          </div>
          {onRetry && (
            <button 
              className="retry-button"
              onClick={() => onRetry(message)}
              aria-label="Retry message"
            >
              Retry
            </button>
          )}
        </div>
      )}
      
      <div className="message-meta">
        <span className="message-time">
          {new Date(message.timestamp).toLocaleTimeString()}
        </span>
        {message.status && (
          <span className={`message-status ${message.status}`}>
            {message.status}
          </span>
        )}
      </div>
    </div>
  );
});

StreamingMessage.displayName = 'StreamingMessage';

export default StreamingMessage;

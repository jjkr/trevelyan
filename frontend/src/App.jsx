import React, { useState } from 'react';
import './App.css';
import logo from './assets/OriginalLogo.png';
import bedrockService from './services/bedrock';

function App() {
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState('');
  const [isLoading, setIsLoading] = useState(false);

  const handleSendMessage = async (e) => {
    e.preventDefault();
    if (input.trim() && !isLoading) {
      const userMessage = input.trim();
      setMessages(prev => [...prev, { text: userMessage, sender: 'user' }]);
      setInput('');
      setIsLoading(true);

      // Add empty AI message that will be updated with streaming content
      const aiMessageIndex = messages.length + 1;
      setMessages(prev => [...prev, { text: '', sender: 'ai', streaming: true }]);

      try {
        await bedrockService.streamMessage(
          userMessage,
          // onChunk callback - append text as it streams
          (chunk) => {
            setMessages(prev => {
              const newMessages = [...prev];
              const aiMessage = newMessages[aiMessageIndex];
              if (aiMessage) {
                aiMessage.text += chunk;
              }
              return newMessages;
            });
          },
          // onError callback
          (error) => {
            setMessages(prev => {
              const newMessages = [...prev];
              newMessages[aiMessageIndex] = {
                text: `Error: ${error}`,
                sender: 'ai',
                error: true
              };
              return newMessages;
            });
          }
        );
      } catch (error) {
        setMessages(prev => {
          const newMessages = [...prev];
          newMessages[aiMessageIndex] = {
            text: `Error: ${error.message || 'Failed to get response'}`,
            sender: 'ai',
            error: true
          };
          return newMessages;
        });
      } finally {
        setIsLoading(false);
        // Remove streaming flag
        setMessages(prev => {
          const newMessages = [...prev];
          const aiMessage = newMessages[aiMessageIndex];
          if (aiMessage) {
            delete aiMessage.streaming;
          }
          return newMessages;
        });
      }
    }
  };

  return (
    <div className="app-container">
      <div className="chat-messages">
        {messages.map((msg, index) => (
          <div key={index} className={`message ${msg.sender} ${msg.error ? 'error' : ''} ${msg.streaming ? 'streaming' : ''}`}>
            {msg.text || (msg.streaming ? 'Thinking...' : '')}
            {msg.streaming && <span className="cursor">|</span>}
          </div>
        ))}
        {isLoading && messages.length > 0 && !messages[messages.length - 1]?.streaming && (
          <div className="message ai loading">
            <span>Connecting to AI...</span>
          </div>
        )}
      </div>
      <form onSubmit={handleSendMessage} className="chat-input-form">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Ask away..."
          className="chat-input"
          disabled={isLoading}
        />
        <button type="submit" className="send-button" disabled={isLoading}>
          <img src={logo} alt="Send" className="send-button-logo" />
        </button>
      </form>
    </div>
  );
}

export default App;

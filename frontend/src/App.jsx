import React, { useState } from 'react';
import './App.css';
import logo from './assets/OriginalLogo.png';

function App() {
  const [messages, setMessages] = useState([]);
  const [input, setInput] = useState('');

  const handleSendMessage = (e) => {
    e.preventDefault();
    if (input.trim()) {
      setMessages([...messages, { text: input, sender: 'user' }]);
      setInput('');
      // Simulate AI response
      setTimeout(() => {
        setMessages((prevMessages) => [
          ...prevMessages,
          { text: `AI response to: "${input}"`, sender: 'ai' },
        ]);
      }, 1000);
    }
  };

  return (
    <div className="app-container">
      <div className="chat-messages">
        {messages.map((msg, index) => (
          <div key={index} className={`message ${msg.sender}`}>
            {msg.text}
          </div>
        ))}
      </div>
      <form onSubmit={handleSendMessage} className="chat-input-form">
        <input
          type="text"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          placeholder="Ask away..."
          className="chat-input"
        />
        <button type="submit" className="send-button">
          <img src={logo} alt="Send" className="send-button-logo" />
        </button>
      </form>
    </div>
  );
}

export default App;

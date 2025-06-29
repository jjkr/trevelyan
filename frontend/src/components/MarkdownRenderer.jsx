import React, { memo } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { oneDark } from 'react-syntax-highlighter/dist/esm/styles/prism';

/**
 * Code block component with syntax highlighting
 */
const CodeBlock = ({ node, inline, className, children, ...props }) => {
  const match = /language-(\w+)/.exec(className || '');
  const language = match ? match[1] : '';

  return !inline && language ? (
    <SyntaxHighlighter
      style={oneDark}
      language={language}
      PreTag="div"
      className="code-block"
      {...props}
    >
      {String(children).replace(/\n$/, '')}
    </SyntaxHighlighter>
  ) : (
    <code className={`inline-code ${className || ''}`} {...props}>
      {children}
    </code>
  );
};

/**
 * Custom components for markdown rendering
 */
const markdownComponents = {
  code: CodeBlock,
  
  // Custom styling for different elements
  h1: ({ children }) => <h1 className="markdown-h1">{children}</h1>,
  h2: ({ children }) => <h2 className="markdown-h2">{children}</h2>,
  h3: ({ children }) => <h3 className="markdown-h3">{children}</h3>,
  h4: ({ children }) => <h4 className="markdown-h4">{children}</h4>,
  h5: ({ children }) => <h5 className="markdown-h5">{children}</h5>,
  h6: ({ children }) => <h6 className="markdown-h6">{children}</h6>,
  
  p: ({ children }) => <p className="markdown-p">{children}</p>,
  
  ul: ({ children }) => <ul className="markdown-ul">{children}</ul>,
  ol: ({ children }) => <ol className="markdown-ol">{children}</ol>,
  li: ({ children }) => <li className="markdown-li">{children}</li>,
  
  blockquote: ({ children }) => <blockquote className="markdown-blockquote">{children}</blockquote>,
  
  table: ({ children }) => <table className="markdown-table">{children}</table>,
  thead: ({ children }) => <thead className="markdown-thead">{children}</thead>,
  tbody: ({ children }) => <tbody className="markdown-tbody">{children}</tbody>,
  tr: ({ children }) => <tr className="markdown-tr">{children}</tr>,
  th: ({ children }) => <th className="markdown-th">{children}</th>,
  td: ({ children }) => <td className="markdown-td">{children}</td>,
  
  a: ({ href, children }) => (
    <a href={href} className="markdown-link" target="_blank" rel="noopener noreferrer">
      {children}
    </a>
  ),
  
  strong: ({ children }) => <strong className="markdown-strong">{children}</strong>,
  em: ({ children }) => <em className="markdown-em">{children}</em>,
  
  hr: () => <hr className="markdown-hr" />,
  
  // Future extensibility: Custom components for agent features
  // toolResult: ToolResultComponent,
  // agentAction: AgentActionComponent,
  // fileAttachment: FileAttachmentComponent,
};

/**
 * Markdown renderer component optimized for chat messages
 */
const MarkdownRenderer = memo(({ content, isStreaming, isCompleted, sender }) => {
  // For user messages, render as plain text to preserve formatting
  if (sender === 'user') {
    return <div className="user-message-text">{content}</div>;
  }

  // For AI messages, render as markdown
  return (
    <div className="ai-message-markdown">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={markdownComponents}
        // Handle streaming content gracefully
        skipHtml={true} // Security: don't render raw HTML
      >
        {content || ''}
      </ReactMarkdown>
      {isStreaming && !isCompleted && (
        <span className="streaming-cursor" aria-label="AI is typing">
          ▋
        </span>
      )}
    </div>
  );
});

MarkdownRenderer.displayName = 'MarkdownRenderer';

export default MarkdownRenderer;

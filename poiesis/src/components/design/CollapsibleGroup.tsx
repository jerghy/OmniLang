import React, { useState } from 'react';

interface CollapsibleGroupProps {
  title: string;
  summary: string;
  defaultOpen?: boolean;
  children: React.ReactNode;
}

export const CollapsibleGroup: React.FC<CollapsibleGroupProps> = ({
  title,
  summary,
  defaultOpen = false,
  children,
}) => {
  const [isOpen, setIsOpen] = useState(defaultOpen);

  const headerStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: '8px',
    padding: '6px 12px',
    borderBottom: '1px solid #e0e0e0',
    cursor: 'pointer',
    userSelect: 'none',
    fontSize: '13px',
    lineHeight: '1.4',
  };

  const chevronStyle: React.CSSProperties = {
    display: 'inline-block',
    fontSize: '12px',
    color: '#666',
    transform: isOpen ? 'rotate(90deg)' : 'rotate(0deg)',
    transition: 'transform 150ms ease',
    flexShrink: 0,
  };

  const titleStyle: React.CSSProperties = {
    fontWeight: 600,
    textTransform: 'uppercase',
    color: '#333',
    whiteSpace: 'nowrap',
  };

  const summaryStyle: React.CSSProperties = {
    color: '#888',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
  };

  const childrenContainerStyle: React.CSSProperties = {
    padding: '4px 12px 8px',
  };

  const [isHovered, setIsHovered] = useState(false);

  const mergedHeaderStyle: React.CSSProperties = {
    ...headerStyle,
    background: isHovered ? '#f5f5f5' : undefined,
  };

  return (
    <div>
      <div
        style={mergedHeaderStyle}
        onClick={() => setIsOpen((prev) => !prev)}
        onMouseEnter={() => setIsHovered(true)}
        onMouseLeave={() => setIsHovered(false)}
        role="button"
        tabIndex={0}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ' ') {
            e.preventDefault();
            setIsOpen((prev) => !prev);
          }
        }}
        aria-expanded={isOpen}
      >
        <span style={chevronStyle}>▶</span>
        <span style={titleStyle}>{title}</span>
        <span style={summaryStyle}>{summary}</span>
      </div>
      {isOpen && <div style={childrenContainerStyle}>{children}</div>}
    </div>
  );
};

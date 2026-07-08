import React from 'react';

interface PropertyRowProps {
  label: string;
  children: React.ReactNode;
  inherited?: boolean;
  showOverrideDot?: boolean;
  onRemove?: () => void;
  onTokenToggle?: () => void;
  hasToken?: boolean;
  tokenName?: string | null;
}

export function PropertyRow({
  label,
  children,
  inherited = false,
  showOverrideDot = false,
  onRemove,
  onTokenToggle,
  hasToken = false,
  tokenName,
}: PropertyRowProps) {
  const rowStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    height: '28px',
    padding: '0 2px',
  };

  const overrideDotStyle: React.CSSProperties = {
    width: '6px',
    height: '6px',
    borderRadius: '50%',
    backgroundColor: 'var(--dp-accent, #6366f1)',
    flexShrink: 0,
  };

  const labelStyle: React.CSSProperties = {
    width: '60px',
    fontSize: '12px',
    color: inherited ? 'var(--dp-text-muted, #a09db8)' : 'var(--dp-text-secondary, #6b6893)',
    overflow: 'hidden',
    textOverflow: 'ellipsis',
    whiteSpace: 'nowrap',
    flexShrink: 1,
    lineHeight: '28px',
  };

  const valueStyle: React.CSSProperties = {
    flex: 1,
    minWidth: 0,
  };

  const iconButtonStyle: React.CSSProperties = {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'none',
    border: 'none',
    cursor: 'pointer',
    padding: '2px',
    fontSize: '12px',
    lineHeight: '1',
    flexShrink: 0,
    borderRadius: '3px',
    transition: 'color 150ms ease, background 150ms ease',
  };

  const tokenColor = hasToken ? 'var(--dp-accent, #6366f1)' : 'var(--dp-text-muted, #a09db8)';

  return (
    <div style={rowStyle}>
      {showOverrideDot && <span style={overrideDotStyle} />}
      <span style={labelStyle} title={label}>
        {label}
      </span>
      <div style={valueStyle}>{children}</div>
      {onTokenToggle && (
        <button
          style={{ ...iconButtonStyle, color: tokenColor }}
          onClick={onTokenToggle}
          title={tokenName ?? undefined}
          type="button"
        >
          {'\u{1F517}'}
        </button>
      )}
      {onRemove && (
        <button
          style={{ ...iconButtonStyle, color: 'var(--dp-text-muted, #a09db8)' }}
          onClick={onRemove}
          title="Remove"
          type="button"
        >
          {'×'}
        </button>
      )}
    </div>
  );
}

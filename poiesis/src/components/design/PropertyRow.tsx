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
    gap: '4px',
    height: '28px',
    padding: '0 4px',
  };

  const overrideDotStyle: React.CSSProperties = {
    width: '6px',
    height: '6px',
    borderRadius: '50%',
    backgroundColor: '#6366f1',
    flexShrink: 0,
  };

  const labelStyle: React.CSSProperties = {
    width: '56px',
    fontSize: '11px',
    color: inherited ? '#94a3b8' : '#374151',
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
    borderRadius: '2px',
  };

  const tokenColor: string = hasToken ? '#6366f1' : '#94a3b8';

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
          style={iconButtonStyle}
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

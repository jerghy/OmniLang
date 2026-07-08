import React, { useEffect, useCallback } from 'react';

export interface FocusOverlayProps {
  visible: boolean;
  onClose: () => void;
  children: React.ReactNode;
  floatingCard: React.ReactNode;
}

const overlayStyle: React.CSSProperties = {
  position: 'absolute',
  inset: 0,
  zIndex: 500,
  backgroundColor: '#f8fafc',
  display: 'flex',
  flexDirection: 'column',
};

const topBarStyle: React.CSSProperties = {
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'space-between',
  padding: '8px 16px',
  borderBottom: '1px solid #e2e8f0',
  fontSize: 14,
  color: '#475569',
};

const closeBtnStyle: React.CSSProperties = {
  background: 'none',
  border: 'none',
  fontSize: 20,
  cursor: 'pointer',
  color: '#64748b',
  padding: '4px 8px',
  borderRadius: 4,
  lineHeight: 1,
};

const centerStyle: React.CSSProperties = {
  flex: 1,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  padding: 32,
  overflow: 'auto',
};

const floatingCardStyle: React.CSSProperties = {
  position: 'absolute',
  bottom: 24,
  right: 24,
  zIndex: 501,
  backgroundColor: '#ffffff',
  border: '1px solid #e2e8f0',
  borderRadius: 8,
  boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
};

export const FocusOverlay: React.FC<FocusOverlayProps> = ({
  visible,
  onClose,
  children,
  floatingCard,
}) => {
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    },
    [onClose]
  );

  useEffect(() => {
    if (!visible) return;
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [visible, handleKeyDown]);

  if (!visible) return null;

  return (
    <div style={overlayStyle}>
      <div style={topBarStyle}>
        <span>聚焦模式 — 拖拽元素边缘直接编辑</span>
        <button style={closeBtnStyle} onClick={onClose} type="button">
          &times;
        </button>
      </div>
      <div style={centerStyle}>{children}</div>
      <div style={floatingCardStyle}>{floatingCard}</div>
    </div>
  );
};

export default FocusOverlay;

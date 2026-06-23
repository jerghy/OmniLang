import React, { useRef, useEffect, useCallback, useState } from 'react';

export interface NidEntry {
  elementId: string;
  tag: string;
  component: string;
}

interface CanvasPreviewProps {
  srcdoc: string;
  onSelect?: (nid: string, entry: NidEntry) => void;
  onRendered?: (nidMap: Record<string, NidEntry>) => void;
  onHover?: (nid: string) => void;
  style?: React.CSSProperties;
  className?: string;
}

export function CanvasPreview({
  srcdoc,
  onSelect,
  onRendered,
  onHover,
  style,
  className,
}: CanvasPreviewProps) {
  const iframeRef = useRef<HTMLIFrameElement>(null);
  const nidMapRef = useRef<Record<string, NidEntry>>({});
  const [loadError, setLoadError] = useState(false);

  // Reset error state when srcdoc changes
  useEffect(() => {
    setLoadError(false);
  }, [srcdoc]);

  // Detect error markers in srcdoc and show fallback UI instead of iframe error page
  useEffect(() => {
    if (srcdoc && srcdoc.includes('Compilation Error')) {
      setLoadError(true);
    }
  }, [srcdoc]);

  useEffect(() => {
    const iframe = iframeRef.current;
    if (!iframe || !srcdoc) return;

    iframe.srcdoc = srcdoc;

    const handler = (e: MessageEvent) => {
      if (e.source !== iframe.contentWindow) return;

      switch (e.data?.type) {
        case 'omni-select': {
          const nid = e.data.nid as string;
          const entry = nidMapRef.current[nid];
          onSelect?.(nid, entry || { elementId: '', tag: '', component: '' });
          // Also highlight in iframe
          iframe.contentWindow?.postMessage({ type: 'omni-highlight', nid }, '*');
          break;
        }
        case 'omni-hover': {
          onHover?.(e.data.nid as string);
          break;
        }
        case 'omni-rendered': {
          nidMapRef.current = (e.data.nidMap || {}) as Record<string, NidEntry>;
          onRendered?.(nidMapRef.current);
          break;
        }
        case 'omni-signal-changed':
        case 'omni-event-fired':
          // Debug messages — log to console for now
          console.debug('[CanvasPreview]', e.data.type, e.data);
          break;
      }
    };

    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [srcdoc, onSelect, onRendered, onHover]);

  const postMessage = useCallback((msg: unknown) => {
    iframeRef.current?.contentWindow?.postMessage(msg, '*');
  }, []);

  if (loadError) {
    return (
      <div style={{
        width: '100%',
        height: '100%',
        padding: 20,
        color: '#ef4444',
        textAlign: 'center',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        backgroundColor: '#fff',
        ...style,
      }}>
        <div>
          <div style={{ fontSize: 18, fontWeight: 600, marginBottom: 8 }}>Compilation Error</div>
          <div style={{ fontSize: 13, color: '#94a3b8' }}>Check the .omni source for syntax errors</div>
        </div>
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      style={{
        width: '100%',
        height: '100%',
        border: 'none',
        backgroundColor: '#fff',
        ...style,
      }}
      className={className}
      sandbox="allow-scripts allow-same-origin"
      title="OmniView Preview"
    />
  );
}

// Expose postMessage capability for parent to call
export type CanvasPreviewHandle = {
  postMessage: (msg: unknown) => void;
  highlight: (nid: string) => void;
  clearHighlight: () => void;
  replaceComponent: (name: string, factoryCode: string) => void;
};

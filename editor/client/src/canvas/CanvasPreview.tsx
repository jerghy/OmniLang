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
  const prevSelectedNid = useRef<string | null>(null);
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
          // Clear previous highlight before setting new one
          if (prevSelectedNid.current && prevSelectedNid.current !== nid) {
            iframe.contentWindow?.postMessage({ type: 'omni-clear-hl' }, window.location.origin);
          }
          prevSelectedNid.current = nid;
          // Highlight the newly selected element
          iframe.contentWindow?.postMessage({ type: 'omni-highlight', nid }, window.location.origin);
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
    iframeRef.current?.contentWindow?.postMessage(msg, window.location.origin);
  }, []);

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

// Reserved for future forwardRef usage — enables parent components to call
// postMessage / highlight / replaceComponent imperatively on the CanvasPreview.
export type CanvasPreviewHandle = {
  postMessage: (msg: unknown) => void;
  highlight: (nid: string) => void;
  clearHighlight: () => void;
  replaceComponent: (name: string, factoryCode: string) => void;
};

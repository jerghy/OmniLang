import React, { useRef, useEffect } from 'react';
import { Logger } from "../../../lib/utils/logger";

const log = new Logger({ prefix: "CanvasPreview" });

export interface NidEntry {
  elementId: string;
  tag: string;
  component: string;
}

interface CanvasPreviewProps {
  srcdoc: string;
  selectedId?: string | null;
  onSelect?: (nid: string, entry: NidEntry) => void;
  onRendered?: (nidMap: Record<string, NidEntry>) => void;
  onHover?: (nid: string) => void;
  style?: React.CSSProperties;
  className?: string;
}

export function CanvasPreview({
  srcdoc,
  selectedId,
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
          // Debug messages — log via Logger
          log.debug(e.data.type, e.data);
          break;
      }
    };

    window.addEventListener('message', handler);
    return () => window.removeEventListener('message', handler);
  }, [srcdoc, onSelect, onRendered, onHover]);

  // Sync selection highlight: when selectedId changes (from tree click), highlight in iframe
  useEffect(() => {
    const iframe = iframeRef.current;
    if (!iframe) return;
    // Clear previous, then highlight the element by ID
    iframe.contentWindow?.postMessage({ type: 'omni-clear-hl' }, window.location.origin);
    if (selectedId) {
      // Build reverse map: elementId → nid
      let targetNid: string | null = null;
      for (const [nid, entry] of Object.entries(nidMapRef.current)) {
        if (entry.elementId === selectedId) { targetNid = nid; break; }
      }
      if (targetNid) {
        iframe.contentWindow?.postMessage({ type: 'omni-highlight', nid: targetNid }, window.location.origin);
      } else {
        // Fallback: try querying the element by id attribute directly
        // The compiled DOM has id="{store_element_id}" so we can find it
        const doc = iframe.contentDocument;
        if (doc) {
          const el = doc.querySelector(`[id="${selectedId}"]`) as HTMLElement | null;
          if (el) {
            const nid = el.dataset.nid;
            if (nid) {
              iframe.contentWindow?.postMessage({ type: 'omni-highlight', nid }, window.location.origin);
            }
          }
        }
      }
    }
  }, [selectedId]);

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

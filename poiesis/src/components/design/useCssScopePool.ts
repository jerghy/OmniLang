// poiesis/src/components/design/useCssScopePool.ts
import { useRef, useCallback, useEffect } from "react";

function hashStr(s: string): number {
  let hash = 0;
  for (let i = 0; i < s.length; i++) {
    const ch = s.charCodeAt(i);
    hash = ((hash << 5) - hash) + ch;
    hash |= 0;
  }
  return hash;
}

export interface UseCssScopePoolResult {
  updateScopes: (css: string, componentCss: Record<string, string>) => void;
}

export function useCssScopePool(): UseCssScopePoolResult {
  const scopeRefs = useRef<Map<string, HTMLStyleElement>>(new Map());
  const lastHashes = useRef<{ design: number; components: Map<string, number> }>({
    design: 0,
    components: new Map(),
  });

  const upsertScope = useCallback((scopeId: string, css: string) => {
    let el = scopeRefs.current.get(scopeId);
    if (!el) {
      el = document.createElement("style");
      el.setAttribute("data-css-scope", scopeId);
      document.head.appendChild(el);
      scopeRefs.current.set(scopeId, el);
    }
    el.textContent = css;
    return el;
  }, []);

  const removeScope = useCallback((scopeId: string) => {
    const el = scopeRefs.current.get(scopeId);
    if (el) {
      el.remove();
      scopeRefs.current.delete(scopeId);
    }
  }, []);

  const updateScopes = useCallback(
    (css: string, componentCss: Record<string, string>) => {
      const designHash = hashStr(css);
      if (designHash !== lastHashes.current.design) {
        upsertScope("design", css);
        lastHashes.current.design = designHash;
      }

      const newComponents = new Set(Object.keys(componentCss));
      for (const [name, compCss] of Object.entries(componentCss)) {
        const hash = hashStr(compCss);
        if (hash !== lastHashes.current.components.get(name)) {
          upsertScope(`component:${name}`, compCss);
          lastHashes.current.components.set(name, hash);
        }
      }

      const stale: string[] = [];
      for (const name of lastHashes.current.components.keys()) {
        if (!newComponents.has(name)) stale.push(name);
      }
      for (const name of stale) {
        removeScope(`component:${name}`);
        lastHashes.current.components.delete(name);
      }
    },
    [upsertScope, removeScope],
  );

  useEffect(() => {
    return () => {
      for (const el of scopeRefs.current.values()) {
        el.remove();
      }
      scopeRefs.current.clear();
    };
  }, []);

  return { updateScopes };
}

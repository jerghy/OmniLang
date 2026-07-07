// components/design/CompilerPreview.tsx
// Renders the OmniView compiler output in a sandboxed iframe, matching
// exactly what `omnic` generates — full CSS, multi-file imports, styles.
// Updates are debounced to avoid excessive recompilation during editing.

import { useEffect, useRef, useState, useCallback } from "react";
import { usePoiesisStore } from "../../store";
import { generateOmniSource } from "../../services/codegen";
import { rustCompiler } from "../../commands/rustCompiler";

/** Debounce delay in ms before recompiling after last change. */
const DEBOUNCE_MS = 400;

export function CompilerPreview() {
  const elements = usePoiesisStore((s) => s.elements);
  const wires = usePoiesisStore((s) => s.wires);
  const rootOrder = usePoiesisStore((s) => s.rootOrder);
  const designPage = usePoiesisStore((s) => s.designPage);

  const iframeRef = useRef<HTMLIFrameElement>(null);
  const [htmlContent, setHtmlContent] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // Stable refs to avoid dependency-array-driven recreation of compile()
  const elementsRef = useRef(elements);
  elementsRef.current = elements;
  const wiresRef = useRef(wires);
  wiresRef.current = wires;
  const rootOrderRef = useRef(rootOrder);
  rootOrderRef.current = rootOrder;

  const activeTab = designPage.openTabs.find((t) => t.id === designPage.activeTabId);
  const componentName = activeTab?.componentName ?? "main";

  const compile = useCallback(async () => {
    const el = elementsRef.current;
    const wi = wiresRef.current;
    const ro = rootOrderRef.current;
    setError(null);
    try {
      const genResult = generateOmniSource({
        designElements: el,
        designWires: wi,
        codeTabs: [{ id: "main", type: "logic", label: "Main", icon: "📄", graph: { elements: el, wires: wi }, variables: [] }],
        componentName,
        rootOrder: ro,
      });

      if (!genResult.value) {
        setHtmlContent(null);
        return;
      }

      await rustCompiler.init();
      if (!rustCompiler.isReady) {
        setError("OmniLang 编译器未加载");
        return;
      }

      const result = rustCompiler.compileToOmniView(genResult.value, true);
      if (!result.success || !result.files) {
        const errMsg = result.errors?.map((e) => e.message).join("; ") || "编译失败";
        setError(errMsg);
        return;
      }

      const html = `<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>${result.files.css}</style>
</head>
<body>
${result.files.html}
<script>${result.files.js}<\/script>
</body>
</html>`;

      setHtmlContent(html);
    } catch (e) {
      setError(String(e));
    }
  }, [componentName]); // stable — only recreated when componentName changes

  useEffect(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    timerRef.current = setTimeout(compile, DEBOUNCE_MS);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [compile]);

  useEffect(() => {
    const iframe = iframeRef.current;
    if (!iframe || !htmlContent) return;
    iframe.srcdoc = htmlContent;
  }, [htmlContent]);

  if (elements.length === 0) {
    return (
      <div
        style={{
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          color: "#94a3b8",
          fontSize: 13,
        }}
      >
        空画布 — 从组件面板拖入元素
      </div>
    );
  }

  if (error) {
    return (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          justifyContent: "center",
          height: "100%",
          color: "#ef4444",
          fontSize: 12,
          padding: 16,
        }}
      >
        <div style={{ fontWeight: 600, marginBottom: 4 }}>编译错误</div>
        <div style={{ color: "#64748b", textAlign: "center" }}>{error}</div>
      </div>
    );
  }

  return (
    <iframe
      ref={iframeRef}
      sandbox="allow-scripts"
      title="compiler-preview"
      style={{
        width: "100%",
        height: "100%",
        border: "none",
        background: "#fff",
      }}
    />
  );
}

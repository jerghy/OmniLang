// CodeBlockEditor.tsx
// Monaco text editor + WASM semantic dependency analysis for code blocks.
// Replaces the SVG block editor (BlockEditor) with a professional text-editing
// experience, backed by OmniCompiler.analyse_dependencies() for live data-flow
// insight in the right-side DependencyPanel.
//
// Design: docs/superpowers/specs/2026-07-26-codeblock-text-editor-design.md

import { useCallback, useEffect, useRef, useState } from "react";
import { Logger } from "../../../lib/utils/logger";
import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";
import tsWorker from "monaco-editor/esm/vs/language/typescript/ts.worker?worker";
import cssWorker from "monaco-editor/esm/vs/language/css/css.worker?worker";
import htmlWorker from "monaco-editor/esm/vs/language/html/html.worker?worker";
import jsonWorker from "monaco-editor/esm/vs/language/json/json.worker?worker";
import { useMonacoEditor } from "../../../services/monaco/useMonacoEditor";
import { getOmniCompiler } from "../../../services/monaco/diagnostics";
import { DependencyPanel } from "./DependencyPanel";
import type { AnalysisStatus } from "./DependencyPanel";
import type { DependencyInfo } from "../../../core/types/dependency-info";
import type { ElementData, WireData, CodeTab } from "../../../core/types";
import styles from "./CodeBlockEditor.module.css";

// Configure Monaco workers for Vite (shared across all Monaco instances)
(self as unknown as { MonacoEnvironment: unknown }).MonacoEnvironment = {
  getWorker(_: string, label: string) {
    if (label === "typescript" || label === "javascript") return new tsWorker();
    if (label === "css") return new cssWorker();
    if (label === "html") return new htmlWorker();
    if (label === "json") return new jsonWorker();
    return new editorWorker();
  },
};

// ── Public Types ──

export interface CodeBlockEditorContext {
  /** Function parameter names from the enclosing fn node */
  fnParams: string[];
  /** Current component name (from active tab) */
  componentName: string;
  /** Relative path to the .omni source file */
  omniSourcePath: string;
  /** Signal names declared in this component / referenced stores */
  signals: string[];
  /** Store names referenced by this component */
  stores: string[];
  /** Full project .omni sources for multi-file WASM analysis */
  allSources: Record<string, string>;
  /** All design elements (for pipeline.ts full-source generation) */
  designElements: ElementData[];
  /** All wires (for pipeline.ts full-source generation) */
  designWires: WireData[];
  /** Code tabs (for pipeline.ts full-source generation) */
  codeTabs: CodeTab[];
}

export interface CodeBlockEditorProps {
  elementId: string;
  initialSource: string;
  /** WASM analysis context — required for dependency analysis */
  context: CodeBlockEditorContext;
  /** Called on save with current source and latest dependency info */
  onSave: (source: string, deps: DependencyInfo | null) => void;
  onClose: () => void;
}

// ── Constants ──

const DEBOUNCE_MS = 500;

// ── Component ──

export function CodeBlockEditor({
  elementId,
  initialSource,
  context,
  onSave,
  onClose,
}: CodeBlockEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const [deps, setDeps] = useState<DependencyInfo | null>(null);
  const [analysisStatus, setAnalysisStatus] = useState<AnalysisStatus>("loading");
  const [panelCollapsed, setPanelCollapsed] = useState(false);
  const sourceRef = useRef(initialSource);
  const depsRef = useRef<DependencyInfo | null>(null);

  // Keep depsRef in sync so keyboard handler reads latest value
  useEffect(() => {
    depsRef.current = deps;
  }, [deps]);

  // ── Monaco editor via shared hook ──────────────────────────────
  const { editor } = useMonacoEditor({
    containerRef,
    initialValue: initialSource,
    language: "omnilang",
    onChange: (value) => {
      sourceRef.current = value;
    },
  });

  // ── WASM analysis: diagnostics + dependency extraction (debounced) ──
  useEffect(() => {
    setAnalysisStatus("loading");

    const timer = setTimeout(async () => {
      try {
        const compiler = await getOmniCompiler();
        if (!compiler) {
          setAnalysisStatus("unavailable");
          return;
        }

        // 1. Generate full .omni source via pipeline.ts for semantic context
        const fullSource = await generateFullOmniSource(
          context.componentName,
          sourceRef.current,
          context,
        );

        // 2. Register with WASM and run diagnostics
        const sourceToAnalyse = fullSource ?? sourceRef.current;
        compiler.upsert_source(context.omniSourcePath, sourceToAnalyse);

        // Syntax diagnostics → Monaco markers
        const diags = compiler.diagnostics();
        const model = editor?.getModel();
        if (model && diags.length > 0) {
          const { applyDiagnostics } = await import(
            "../../../services/monaco/diagnostics"
          );
          await applyDiagnostics(model, diags, "omnilang");
        }

        // 3. Semantic dependency analysis → DependencyPanel
        const result = compiler.analyse_dependencies();
        if (result) {
          const depInfo = mapSemanticResult(result, elementId);
          const filtered = filterRelevantDeps(depInfo, context);
          setDeps(filtered);
          setAnalysisStatus("ready");
        } else {
          // analyse_dependencies returned null/undefined
          setAnalysisStatus("unavailable");
        }
      } catch (err) {
        new Logger({ prefix: "CodeBlockEditor" }).warn("[CodeBlockEditor] WASM analysis failed:", err);
        setAnalysisStatus("unavailable");
      }
    }, DEBOUNCE_MS);

    return () => clearTimeout(timer);
    // getValue intentionally excluded from deps — its reference changes every render;
    // we read sourceRef.current for the latest value instead.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [editor, context, elementId]);

  // ── Keyboard: Ctrl+Enter save, Escape cancel ───────────────────
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
      if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        onSave(sourceRef.current, depsRef.current);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onSave, onClose]);

  // ── Save handler ───────────────────────────────────────────────
  const handleSave = useCallback(() => {
    onSave(sourceRef.current, deps);
  }, [onSave, deps]);

  return (
    <div className={styles.overlay} onClick={onClose}>
      <div className={styles.editor} onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className={styles.header}>
          <h3 className={styles.title}>编辑代码块</h3>
          <div className={styles.actions}>
            <button
              type="button"
              className={`${styles.btn} ${styles.btnToggle}`}
              onClick={() => setPanelCollapsed((p) => !p)}
              title={panelCollapsed ? "显示依赖面板" : "隐藏依赖面板"}
            >
              {panelCollapsed ? "☰ 依赖" : "☰"}
            </button>
            <button
              type="button"
              className={`${styles.btn} ${styles.btnSave}`}
              onClick={handleSave}
            >
              保存
            </button>
            <button
              type="button"
              className={`${styles.btn} ${styles.btnCancel}`}
              onClick={onClose}
            >
              取消
            </button>
          </div>
        </div>

        {/* Body: Monaco + DependencyPanel */}
        <div className={styles.body}>
          <div ref={containerRef} className={styles.monacoArea} />
          {!panelCollapsed && <DependencyPanel deps={deps} status={analysisStatus} />}
        </div>
      </div>
    </div>
  );
}

// ── Helpers ──

interface WasmSymbol {
  name: string;
  kind: string;
  type_ann?: string;
  mutable: boolean;
}

interface WasmSemanticResult {
  symbols: WasmSymbol[];
  dependencies: Record<string, string[]>;
  diagnostics: unknown[];
}

/**
 * Generate the full .omni source for the current component via pipeline.ts.
 * This gives WASM semantic analysis the complete context (signals, store refs,
 * function params) needed for accurate dependency extraction.
 *
 * Falls back to null if pipeline.ts is unavailable or generation fails,
 * in which case the raw code-block source is used directly.
 */
async function generateFullOmniSource(
  componentName: string,
  _codeBlockSource: string,
  context: CodeBlockEditorContext,
): Promise<string | null> {
  try {
    const { generateOmniSource } = await import(
      "../../../services/codegen/pipeline"
    );
    const result = generateOmniSource({
      componentName,
      designElements: context.designElements,
      designWires: context.designWires,
      codeTabs: context.codeTabs,
    });
    return result.value;
  } catch {
    return null;
  }
}

/**
 * Filter WASM semantic results to only show dependencies relevant to
 * the current code block's context (signals, stores, params).
 *
 * This prevents the panel from showing every symbol in the component
 * and instead focuses on what this specific code block reads/writes/calls.
 */
function filterRelevantDeps(
  deps: DependencyInfo,
  context: CodeBlockEditorContext,
): DependencyInfo {
  const signalSet = new Set(context.signals);
  const storeSet = new Set(context.stores);
  const paramSet = new Set(context.fnParams);

  const isRelevant = (name: string, kind: string): boolean => {
    if (kind === "param") return paramSet.has(name);
    if (kind === "signal" || kind === "computed") return signalSet.has(name);
    if (kind === "store") return storeSet.has(name);
    // locals and functions are always relevant within their code block
    if (kind === "local" || kind === "function") return true;
    return false;
  };

  return {
    codeBlockPid: deps.codeBlockPid,
    reads: deps.reads.filter((e) => isRelevant(e.name, e.kind)),
    writes: deps.writes.filter((e) => isRelevant(e.name, e.kind)),
    calls: deps.calls.filter((e) => isRelevant(e.storeName, "store")),
  };
}

/** Map WASM SemanticResult to DependencyInfo for the DependencyPanel. */
function mapSemanticResult(
  result: WasmSemanticResult,
  _codeBlockPid: string,
): DependencyInfo {
  const symbols = result.symbols ?? [];
  const deps = result.dependencies ?? {};

  const reads: DependencyInfo["reads"] = [];
  const writes: DependencyInfo["writes"] = [];
  const calls: DependencyInfo["calls"] = [];

  // Build a set of names that are dependencies of any symbol
  const allDeps = new Set(Object.values(deps).flat());

  for (const sym of symbols) {
    const kind = normalizeKind(sym.kind);
    const entry = {
      name: sym.name,
      kind,
      sourceLine: 0, // WASM result doesn't include line info yet
    };

    // Heuristic: if a symbol appears as a dependency of another symbol,
    // it's being read; if it's mutable, it's also being written
    const isRead = allDeps.has(sym.name);

    if (sym.kind === "store" || sym.kind === "action") {
      calls.push({
        storeName: sym.name,
        sourceLine: 0,
      });
    } else {
      if (isRead || kind === "param" || kind === "prop" || kind === "signal" || kind === "computed") {
        reads.push(entry);
      }
      if (sym.mutable) {
        writes.push(entry);
      }
    }
  }

  return {
    codeBlockPid: _codeBlockPid,
    reads,
    writes,
    calls,
  };
}

/** Normalize WASM kind strings to DependencyPanel DepEntry.kind. */
function normalizeKind(kind: string): DependencyInfo["reads"][number]["kind"] {
  const lower = kind.toLowerCase();
  const map: Record<string, DependencyInfo["reads"][number]["kind"]> = {
    variable: "local",
    signal: "signal",
    computed: "computed",
    prop: "prop",
    store: "store",
    param: "param",
    function: "function",
    action: "store",
    getter: "computed",
    event: "function",
    ref: "unknown",
    component: "unknown",
    unknown: "unknown",
  };
  return map[lower] ?? "unknown";
}

import React, { useState, useCallback, useRef, useEffect, useMemo } from "react";
import { usePoiesisStore } from "../../store";
import { useCanvasStore } from "../canvas/canvas-store";
import type { DefEntry, ComputedEntry } from "../../core/types/project-json";
import type { StoreDefinition } from "../../core/types/store";
import { parseDebugValue } from "../../services/debug/parse-debug-value";
import { analyzeSignalUsage } from "../../services/debug/signal-analysis";
import type { SignalUsage } from "../../services/debug/signal-analysis";
import { synthesizeConditionValue } from "../../services/debug/synthesize-condition";
import { extractAssignSamples } from "../../services/debug/sample-extractor";
import { generateOmniFragment } from "../../services/codegen/pipeline";

// ── OmniLang common types for dropdown ──
const OMNI_TYPES = [
  "String", "i32", "i64", "f64", "f32",
  "bool", "Array<String>", "Array<i32>", "Array<f64>",
  "Map<String, String>",
];

// ── Panel tabs ──
type PanelTab = "interface" | "signals" | "definitions";

// ── Styles using --ol-* tokens to match Poiesis visual language ──
//     Matches StoreMembersPanel for visual consistency.

// addBtn 基础样式独立常量：resetBtn/stepBtn 复用（S 字面量内不能自引用）
const ADD_BTN_BASE: React.CSSProperties = {
  padding: "1px 7px",
  fontSize: "var(--ol-font-size-11)",
  fontWeight: "var(--ol-font-weight-600)",
  border: "1px solid var(--ol-border)",
  borderRadius: "var(--ol-radius-sm)",
  background: "var(--ol-surface)",
  color: "var(--ol-text-secondary)",
  cursor: "pointer",
  lineHeight: "18px",
};

const S = {
  panel: {
    position: "absolute" as const,
    width: 260,
    background: "var(--ol-surface)",
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-lg)",
    boxShadow: "0 4px 16px rgba(0, 0, 0, 0.08)",
    fontSize: "var(--ol-font-size-12)",
    zIndex: 10,
    userSelect: "none" as const,
    overflow: "hidden",
  },

  panelHeader: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "8px 12px",
    background: "var(--ol-gray-50)",
    borderBottom: "1px solid var(--ol-border)",
    cursor: "grab",
    fontWeight: "var(--ol-font-weight-600)",
    fontSize: "var(--ol-font-size-12)",
    color: "var(--ol-text)",
  } as React.CSSProperties,

  panelHeaderTitle: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1p5)",
  } as React.CSSProperties,

  collapseBtn: {
    background: "none",
    border: "none",
    cursor: "pointer",
    color: "var(--ol-text-secondary)",
    fontSize: "var(--ol-font-size-14)",
    padding: "0 2px",
    lineHeight: 1,
  } as React.CSSProperties,

  panelBody: {
    maxHeight: 400,
    overflowY: "auto" as const,
    padding: 0,
  } as React.CSSProperties,

  section: {
    borderBottom: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  sectionHeader: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "8px 12px",
    fontWeight: "var(--ol-font-weight-600)",
    fontSize: "var(--ol-font-size-11)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
  } as React.CSSProperties,

  addBtn: { ...ADD_BTN_BASE } as React.CSSProperties,

  entryRow: {
    display: "flex",
    alignItems: "center",
    padding: "5px 12px",
    gap: "var(--ol-space-1p5)",
    fontSize: "var(--ol-font-size-12)",
    color: "var(--ol-text)",
    borderTop: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  entryName: {
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text)",
    flexShrink: 0,
  } as React.CSSProperties,

  entrySep: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
  } as React.CSSProperties,

  entryType: {
    color: "var(--ol-text-secondary)",
    fontSize: "var(--ol-font-size-11)",
    flexShrink: 0,
  } as React.CSSProperties,

  entryDefault: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    flex: 1,
    minWidth: 0,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap" as const,
  } as React.CSSProperties,

  deleteBtn: {
    background: "none",
    border: "none",
    color: "var(--ol-text-muted)",
    cursor: "pointer",
    fontSize: "var(--ol-font-size-13)",
    padding: "0 2px",
    lineHeight: 1,
    flexShrink: 0,
  } as React.CSSProperties,

  input: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
    outline: "2px solid transparent",
  } as React.CSSProperties,

  emptyState: {
    padding: "10px 12px",
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-12)",
    fontStyle: "italic",
  } as React.CSSProperties,

  // Debug 总开关（🔧）
  debugToggle: (active: boolean): React.CSSProperties => ({
    background: active ? "var(--ol-color-primary)" : "none",
    color: active ? "var(--ol-text-inverse)" : "var(--ol-text-secondary)",
    border: active ? "1px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    cursor: "pointer",
    fontSize: "var(--ol-font-size-11)",
    padding: "1px 6px",
    lineHeight: 1.4,
  }),

  // Tab 栏
  tabBar: {
    display: "flex",
    borderBottom: "1px solid var(--ol-border)",
    background: "var(--ol-gray-50)",
  } as React.CSSProperties,

  tabButton: (active: boolean): React.CSSProperties => ({
    flex: 1,
    padding: "6px 4px",
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    border: "none",
    borderBottom: active ? "2px solid var(--ol-color-primary)" : "2px solid transparent",
    background: "none",
    color: active ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
    cursor: "pointer",
    whiteSpace: "nowrap",
  }),

  placeholder: {
    padding: "16px 12px",
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    fontStyle: "italic",
  } as React.CSSProperties,

  // Add form
  addForm: {
    display: "flex",
    gap: "var(--ol-space-1)",
    padding: "4px 12px",
    alignItems: "center",
    borderTop: "1px solid var(--ol-border-subtle)",
  } as React.CSSProperties,

  addFormConfirm: {
    background: "var(--ol-color-primary)",
    color: "var(--ol-text-inverse)",
    border: "none",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 6px",
    fontSize: "var(--ol-font-size-11)",
    cursor: "pointer",
  } as React.CSSProperties,

  addFormCancel: {
    background: "none",
    border: "none",
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    cursor: "pointer",
  } as React.CSSProperties,

  // 信号调试 Tab
  tabHeaderRow: {
    display: "flex",
    alignItems: "center",
    justifyContent: "space-between",
    padding: "8px 12px",
  } as React.CSSProperties,

  tabHeaderTitle: {
    fontSize: "var(--ol-font-size-11)",
    fontWeight: "var(--ol-font-weight-600)",
    color: "var(--ol-text-secondary)",
    textTransform: "uppercase",
    letterSpacing: "0.05em",
  } as React.CSSProperties,

  resetBtn: { ...ADD_BTN_BASE } as React.CSSProperties,

  debugRow: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1p5)",
    padding: "5px 12px",
    borderTop: "1px solid var(--ol-border-subtle)",
    fontSize: "var(--ol-font-size-12)",
  } as React.CSSProperties,

  debugName: {
    fontWeight: "var(--ol-font-weight-600)",
    flexShrink: 0,
    maxWidth: 96,
    overflow: "hidden",
    textOverflow: "ellipsis",
  } as React.CSSProperties,

  debugType: {
    color: "var(--ol-text-secondary)",
    fontSize: "var(--ol-font-size-11)",
    flexShrink: 0,
  } as React.CSSProperties,

  debugValueInput: {
    flex: 1,
    minWidth: 0,
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "2px 6px",
    fontSize: "var(--ol-font-size-12)",
    fontFamily: "var(--ol-font-family-mono)",
    background: "var(--ol-surface)",
    color: "var(--ol-text)",
  } as React.CSSProperties,

  invalidInput: {
    borderColor: "var(--ol-color-danger)",
    background: "var(--ol-color-danger-subtle)",
  } as React.CSSProperties,

  stepBtn: { ...ADD_BTN_BASE, padding: "0 6px" } as React.CSSProperties,

  // ── 界面信号卡（Task 10）──
  signalCard: {
    border: "1px solid var(--ol-border)",
    borderRadius: "var(--ol-radius-sm)",
    padding: "6px 10px",
    background: "var(--ol-gray-50)",
    margin: "0 8px 8px",
  } as React.CSSProperties,

  signalCardHeader: {
    display: "flex",
    alignItems: "center",
    gap: "var(--ol-space-1p5)",
  } as React.CSSProperties,

  badgeText: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-10)",
    flexShrink: 0,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap" as const,
  } as React.CSSProperties,

  branchGroup: {
    display: "flex",
    flexDirection: "column",
    gap: "3px",
    marginTop: "6px",
  } as React.CSSProperties,

  branchRow: {
    display: "flex",
    alignItems: "center",
    gap: "4px",
    flexWrap: "wrap",
  } as React.CSSProperties,

  chainPrefix: {
    fontFamily: "var(--ol-font-family-mono)",
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    flexShrink: 0,
  } as React.CSSProperties,

  branchChipWrap: {} as React.CSSProperties,

  branchChip: (active: boolean): React.CSSProperties => ({
    border: "1px solid var(--ol-border)",
    borderRadius: "999px",
    padding: "1px 8px",
    fontSize: "var(--ol-font-size-11)",
    fontFamily: "var(--ol-font-family-mono)",
    background: active ? "var(--ol-color-primary)" : "var(--ol-surface)",
    color: active ? "var(--ol-text-inverse)" : "var(--ol-text-secondary)",
    cursor: "pointer",
    lineHeight: "16px",
    maxWidth: 150,
    overflow: "hidden",
    textOverflow: "ellipsis",
    whiteSpace: "nowrap" as const,
  }),

  chipRow: {
    display: "flex",
    gap: "4px",
    marginTop: "4px",
    flexWrap: "wrap",
  } as React.CSSProperties,

  quickChip: {
    border: "1px solid var(--ol-border)",
    borderRadius: "999px",
    padding: "0 8px",
    fontSize: "var(--ol-font-size-11)",
    background: "var(--ol-surface)",
    color: "var(--ol-text-secondary)",
    cursor: "pointer",
    lineHeight: "16px",
  } as React.CSSProperties,

  loopGroup: {
    display: "flex",
    flexDirection: "column",
    gap: "3px",
    marginTop: "6px",
  } as React.CSSProperties,

  loopRow: {
    display: "flex",
    alignItems: "center",
    gap: "6px",
  } as React.CSSProperties,

  loopText: {
    fontFamily: "var(--ol-font-family-mono)",
    fontSize: "var(--ol-font-size-11)",
    color: "var(--ol-text-secondary)",
  } as React.CSSProperties,

  loopEmptyTag: {
    fontSize: "var(--ol-font-size-10)",
    color: "var(--ol-text-inverse)",
    background: "var(--ol-color-primary)",
    borderRadius: "4px",
    padding: "0 4px",
    lineHeight: "14px",
    flexShrink: 0,
  } as React.CSSProperties,

  loopTransformTag: {
    fontSize: "var(--ol-font-size-10)",
    color: "var(--ol-text-secondary)",
    border: "1px solid var(--ol-border)",
    borderRadius: "4px",
    padding: "0 4px",
    lineHeight: "14px",
    flexShrink: 0,
  } as React.CSSProperties,

  valueLabel: {
    color: "var(--ol-text-muted)",
    fontSize: "var(--ol-font-size-11)",
    flexShrink: 0,
    width: 16,
  } as React.CSSProperties,

  invalidHint: {
    color: "var(--ol-color-danger)",
    fontSize: "var(--ol-font-size-10)",
    flexShrink: 0,
  } as React.CSSProperties,
};

/** Generate a simple PID (unique within component) */
function generatePid(): string {
  return `pid-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

// 稳定空数组：避免 def 缺失时每次渲染重建 [] 使 useMemo 依赖失效
const EMPTY_SIGNALS: DefEntry[] = [];

export function ComponentDefPanel() {
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const storeDefs = usePoiesisStore((s) => s.storeDefs);
  const inlineStoreDefs = usePoiesisStore((s) => s.inlineStoreDefs);
  const addComponentSignal = usePoiesisStore((s) => s.addComponentSignal);
  const removeComponentSignal = usePoiesisStore((s) => s.removeComponentSignal);
  const updateComponentSignal = usePoiesisStore((s) => s.updateComponentSignal);
  const addComponentProp = usePoiesisStore((s) => s.addComponentProp);
  const removeComponentProp = usePoiesisStore((s) => s.removeComponentProp);
  const updateComponentProp = usePoiesisStore((s) => s.updateComponentProp);
  const addComponentComputed = usePoiesisStore((s) => s.addComponentComputed);
  const removeComponentComputed = usePoiesisStore((s) => s.removeComponentComputed);
  const updateComponentComputed = usePoiesisStore((s) => s.updateComponentComputed);

  // Debug 总开关：on → 调试值生效；off → 暂停（调试值保留在 store 中）
  const debugOverridesEnabled = usePoiesisStore((s) => s.debugOverridesEnabled);
  const setDebugOverridesEnabled = usePoiesisStore((s) => s.setDebugOverridesEnabled);
  const clearDebugValues = usePoiesisStore((s) => s.clearDebugValues);

  const panelPosition = useCanvasStore((s) => s.panelPosition);
  const isCollapsed = useCanvasStore((s) => s.isPanelCollapsed);
  const setPanelPosition = useCanvasStore((s) => s.setPanelPosition);
  const toggleCollapsed = useCanvasStore((s) => s.togglePanelCollapsed);

  // Drag state
  const dragRef = useRef<{ startX: number; startY: number; posX: number; posY: number } | null>(null);
  const [isAdding, setIsAdding] = useState<"prop" | "signal" | "computed" | null>(null);
  const [editingPid, setEditingPid] = useState<string | null>(null);
  // 面板 Tab：默认界面信号调试
  const [tab, setTab] = useState<PanelTab>("interface");

  // Get current component name
  const activeTab = openTabs.find((t) => t.id === activeTabId);
  const componentName = activeTab?.componentName || "main";
  const def = componentDefs[componentName];
  const signals = def?.signals ?? EMPTY_SIGNALS;
  const props = def?.props || [];
  const computeds = def?.computeds || [];

  // 面板内惰性生成组件源码（Task 10）：componentDefs 保存后带 elements/wires
  //（useSaveProject 第 5 步写入）；无 def 或生成失败 → 空串（analyzeSignalUsage
  // 兜底把全部信号归入 dataSignals）。generateOmniFragment 对空元素集短路返回空。
  const getComponentSource = useCallback(() => {
    const compDef = componentDefs[componentName];
    if (!compDef) return "";
    // 单趟收集本组件的内联 store（"Comp.Store" 前缀）
    const inlineStores: StoreDefinition[] = [];
    for (const [key, def] of Object.entries(inlineStoreDefs)) {
      if (key.startsWith(`${componentName}.`)) inlineStores.push(def);
    }
    const result = generateOmniFragment({
      componentName,
      designElements: compDef.elements || [],
      designWires: compDef.wires || [],
      codeTabs: [{ id: "fragment", type: "logic", label: "Fragment", icon: "📄",
        graph: { elements: compDef.elements || [], wires: compDef.wires || [] }, variables: [] }],
      rootOrder: compDef.rootOrder,
      fromDefinition: true,
      inlineStores,
    });
    return result.value ?? "";
  }, [componentDefs, componentName, inlineStoreDefs]);

  // fn 节点源码（列表样本提取用）：fn 是 codeblock + tag==="fn"（ElementData 无
  // "fn" nodeType，brief 的 nodeType === "fn" 恒不命中，见 task-10-report.md）
  const fnSources = useMemo(() => {
    const compDef = componentDefs[componentName];
    const sources: string[] = [];
    for (const el of compDef?.elements || []) {
      if (el.nodeType === "codeblock" && el.tag === "fn") sources.push(el.source);
    }
    return sources;
  }, [componentDefs, componentName]);

  // 信号分析：组件源码 → interfaceSignals（@if/@for 驱动）+ dataSignals。
  // 惰性：仅 Tab 需要分析时生成源码（definitions 跳过）；
  // store 信号收集内联 store（"Comp.Store"）+ 独立 store（ownerKey 限定）。
  const { interfaceSignals, dataSignals } = useMemo((): {
    interfaceSignals: SignalUsage[];
    dataSignals: SignalUsage[];
  } => {
    if (tab === "definitions") return { interfaceSignals: [], dataSignals: [] };
    const signalDecls = signals.map((s) => ({ key: s.name, type: s.type }));
    const storeRefs: Array<{ key: string; type: string }> = [];
    for (const [storeKey, storeDef] of Object.entries(inlineStoreDefs)) {
      for (const sig of storeDef.signals || []) {
        storeRefs.push({ key: `${storeKey}.${sig.name}`, type: sig.type });
      }
    }
    for (const [storeName, storeDef] of Object.entries(storeDefs)) {
      for (const sig of storeDef.signals || []) {
        storeRefs.push({ key: `${storeName}.${sig.name}`, type: sig.type });
      }
    }
    const src = getComponentSource();
    return analyzeSignalUsage(src, signalDecls, storeRefs);
  }, [signals, inlineStoreDefs, storeDefs, getComponentSource, tab]);

  // Drag handling
  const handlePointerDown = useCallback(
    (e: React.PointerEvent) => {
      dragRef.current = {
        startX: e.clientX,
        startY: e.clientY,
        posX: panelPosition.x,
        posY: panelPosition.y,
      };
      (e.target as HTMLElement).setPointerCapture(e.pointerId);
      e.stopPropagation();
    },
    [panelPosition],
  );

  const handlePointerMove = useCallback((e: React.PointerEvent) => {
    if (!dragRef.current) return;
    const dx = e.clientX - dragRef.current.startX;
    const dy = e.clientY - dragRef.current.startY;
    setPanelPosition(dragRef.current.posX + dx, dragRef.current.posY + dy);
  }, [setPanelPosition]);

  const handlePointerUp = useCallback(() => {
    dragRef.current = null;
  }, []);

  // Handle add entry
  const handleAdd = (kind: "prop" | "signal" | "computed", name: string, type: string, extra?: string) => {
    if (!name.trim()) return;
    const pid = generatePid();
    if (kind === "prop") {
      addComponentProp(componentName, { pid, name, type });
    } else if (kind === "signal") {
      addComponentSignal(componentName, { pid, name, type });
    } else {
      addComponentComputed(componentName, { pid, name, returnType: type, expression: extra || "" });
    }
    setIsAdding(null);
  };

  // Inline editing — double-click name to start editing
  const handleDoubleClick = useCallback((pid: string) => {
    setEditingPid(pid);
  }, []);

  const handleEditSave = useCallback((kind: "prop" | "signal" | "computed", pid: string, field: string, value: string) => {
    if (kind === "computed") {
      const updates: Partial<ComputedEntry> = {};
      if (field === "name") updates.name = value;
      if (field === "returnType") updates.returnType = value;
      if (field === "expression") updates.expression = value;
      updateComponentComputed(componentName, pid, updates);
    } else {
      const updates: Partial<DefEntry> = {};
      if (field === "name") updates.name = value;
      if (field === "type") updates.type = value;
      if (field === "defaultValue") updates.defaultValue = value;
      if (kind === "prop") {
        updateComponentProp(componentName, pid, updates);
      } else {
        updateComponentSignal(componentName, pid, updates);
      }
    }
    setEditingPid(null);
  }, [componentName, updateComponentComputed, updateComponentProp, updateComponentSignal]);

  // Delete entry
  const handleDelete = useCallback((kind: "prop" | "signal" | "computed", pid: string) => {
    if (kind === "prop") removeComponentProp(componentName, pid);
    else if (kind === "signal") removeComponentSignal(componentName, pid);
    else removeComponentComputed(componentName, pid);
  }, [componentName, removeComponentProp, removeComponentSignal, removeComponentComputed]);

  // Stabilized renderRow callbacks for memoized DefSection children
  const renderPropsRow = useCallback((entry: DefEntry | ComputedEntry) => {
    const p = entry as DefEntry;
    return (
      <DefEntryRow key={p.pid} entry={p} kind="prop"
        isEditing={editingPid === p.pid} onDoubleClick={() => handleDoubleClick(p.pid)}
        onEditSave={(f, v) => handleEditSave("prop", p.pid, f, v)}
        onDelete={() => handleDelete("prop", p.pid)} />
    );
  }, [editingPid, handleDoubleClick, handleEditSave, handleDelete]);

  const renderSignalsRow = useCallback((entry: DefEntry | ComputedEntry) => {
    const s = entry as DefEntry;
    return (
      <DefEntryRow key={s.pid} entry={s} kind="signal"
        isEditing={editingPid === s.pid}
        onDoubleClick={() => handleDoubleClick(s.pid)}
        onEditSave={(f, v) => handleEditSave("signal", s.pid, f, v)}
        onDelete={() => handleDelete("signal", s.pid)}
      />
    );
  }, [editingPid, handleDoubleClick, handleEditSave, handleDelete]);

  const renderComputedsRow = useCallback((entry: DefEntry | ComputedEntry) => {
    const c = entry as ComputedEntry;
    return (
      <ComputedEntryRow key={c.pid} entry={c}
        isEditing={editingPid === c.pid} onDoubleClick={() => handleDoubleClick(c.pid)}
        onEditSave={(f, v) => handleEditSave("computed", c.pid, f, v)}
        onDelete={() => handleDelete("computed", c.pid)} />
    );
  }, [editingPid, handleDoubleClick, handleEditSave, handleDelete]);

  return (
    <div
      style={{
        ...S.panel,
        left: panelPosition.x,
        top: panelPosition.y,
      }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {/* Header */}
      <div
        style={S.panelHeader}
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <span style={S.panelHeaderTitle}>{componentName}</span>
        <button type="button"
          style={S.debugToggle(debugOverridesEnabled)}
          onClick={() => setDebugOverridesEnabled(!debugOverridesEnabled)}
          title="调试总开关：off 时调试值不参与编译（对比原始渲染）"
        >
          {debugOverridesEnabled ? "🔧 ON" : "🔧 OFF"}
        </button>
        <button type="button" style={S.collapseBtn} onClick={toggleCollapsed}>
          {isCollapsed ? "□" : "－"}
        </button>
      </div>

      {!isCollapsed && (
        <>
          {/* Tab 栏 */}
          <div style={S.tabBar} role="tablist">
            {([
              ["interface", "界面信号调试"],
              ["signals", "信号调试"],
              ["definitions", "定义"],
            ] as Array<[PanelTab, string]>).map(([id, label]) => (
              <button key={id} type="button" role="tab" aria-selected={tab === id}
                style={S.tabButton(tab === id)} onClick={() => setTab(id)}>
                {label}
              </button>
            ))}
          </div>

          {/* Tab 内容 */}
          {tab === "definitions" ? (
            <div style={S.panelBody}>
              <DefSection
                title="Props" sectionKind="prop"
                entries={props} isAdding={isAdding === "prop"}
                onAddToggle={() => setIsAdding(isAdding === "prop" ? null : "prop")}
                onAdd={handleAdd} onCancelAdd={() => setIsAdding(null)}
                renderRow={renderPropsRow}
                emptyText="No props defined"
              />

              <DefSection
                title="Signals" sectionKind="signal"
                entries={signals} isAdding={isAdding === "signal"}
                onAddToggle={() => setIsAdding(isAdding === "signal" ? null : "signal")}
                onAdd={handleAdd} onCancelAdd={() => setIsAdding(null)}
                renderRow={renderSignalsRow}
                emptyText="No signals defined"
              />

              <DefSection
                title="Computed" sectionKind="computed"
                entries={computeds} isAdding={isAdding === "computed"}
                onAddToggle={() => setIsAdding(isAdding === "computed" ? null : "computed")}
                onAdd={handleAdd} onCancelAdd={() => setIsAdding(null)}
                renderRow={renderComputedsRow}
                emptyText="No computeds defined"
              />
            </div>
          ) : tab === "signals" ? (
            <div style={S.panelBody} data-testid="signals-tab">
              <div style={S.tabHeaderRow}>
                <span style={S.tabHeaderTitle}>🎛 信号调试</span>
                <button type="button" style={S.resetBtn} onClick={clearDebugValues}>↺ 重置全部</button>
              </div>
              {dataSignals.map((u) => (
                <SignalDebugRow key={u.key} ownerKey={componentName} usage={u} />
              ))}
              {dataSignals.length === 0 && <p style={S.placeholder}>无数据信号</p>}
            </div>
          ) : (
            <div style={S.panelBody} data-testid="interface-tab">
              <div style={S.tabHeaderRow}>
                <span style={S.tabHeaderTitle}>🎛 界面信号调试</span>
                <button type="button" style={S.resetBtn} onClick={clearDebugValues}>↺ 重置全部</button>
              </div>
              {interfaceSignals.map((u) => (
                <InterfaceSignalCard key={u.key} ownerKey={componentName} usage={u} fnSources={fnSources} />
              ))}
              {interfaceSignals.length === 0 && <p style={S.placeholder}>无界面驱动信号（@if/@for 未用信号）</p>}
            </div>
          )}
        </>
      )}
    </div>
  );
}

// ── Sub-components ──

interface DefEntryRowProps {
  entry: DefEntry;
  kind: "prop" | "signal";
  isEditing: boolean;
  onDoubleClick: () => void;
  onEditSave: (field: string, value: string) => void;
  onDelete: () => void;
}

function DefEntryRow({ entry, kind, isEditing, onDoubleClick, onEditSave, onDelete }: DefEntryRowProps) {
  if (isEditing) {
    return (
      <div style={S.entryRow}>
        <input
          aria-label="Entry name"
          style={{ ...S.input, width: 72, fontWeight: "var(--ol-font-weight-600)" }}
          defaultValue={entry.name}
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span style={S.entrySep}>:</span>
        <select
          aria-label="Entry type"
          style={{ ...S.input, width: 128, cursor: "pointer" }}
          defaultValue={entry.type}
          onBlur={(e) => onEditSave("type", e.target.value)}
          onChange={(e) => onEditSave("type", e.target.value)}
        >
          {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          {!OMNI_TYPES.includes(entry.type) && <option value={entry.type}>{entry.type}</option>}
        </select>
        {kind === "signal" && (
          <>
            <span style={S.entrySep}>=</span>
            <input
              aria-label="Default value"
              style={{ ...S.input, flex: 1 }}
              defaultValue={String(entry.defaultValue ?? "")}
              placeholder="Default"
              onBlur={(e) => onEditSave("defaultValue", e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") onEditSave("defaultValue", e.currentTarget.value); }}
            />
          </>
        )}
        <button type="button" style={S.deleteBtn} onClick={onDelete}>×</button>
      </div>
    );
  }

  return (
    <div style={S.entryRow} onDoubleClick={onDoubleClick}>
      <span style={S.entryName}>{entry.name}</span>
      <span style={S.entrySep}>:</span>
      <span style={S.entryType}>{entry.type}</span>
      <span style={S.entrySep}>=</span>
      <span style={S.entryDefault}>{String(entry.defaultValue ?? "")}</span>
      <button type="button" style={S.deleteBtn} onClick={onDelete}>×</button>
    </div>
  );
}

interface ComputedEntryRowProps {
  entry: ComputedEntry;
  isEditing: boolean;
  onDoubleClick: () => void;
  onEditSave: (field: string, value: string) => void;
  onDelete: () => void;
}

function ComputedEntryRow({ entry, isEditing, onDoubleClick, onEditSave, onDelete }: ComputedEntryRowProps) {
  if (isEditing) {
    return (
      <div style={S.entryRow}>
        <input
          aria-label="Computed name"
          style={{ ...S.input, width: 72, fontWeight: "var(--ol-font-weight-600)" }}
          defaultValue={entry.name}
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span style={S.entrySep}>:</span>
        <select
          aria-label="Computed return type"
          style={{ ...S.input, width: 128, cursor: "pointer" }}
          defaultValue={entry.returnType}
          onBlur={(e) => onEditSave("returnType", e.target.value)}
          onChange={(e) => onEditSave("returnType", e.target.value)}
        >
          {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
          {!OMNI_TYPES.includes(entry.returnType) && <option value={entry.returnType}>{entry.returnType}</option>}
        </select>
        <button type="button" style={S.deleteBtn} onClick={onDelete}>×</button>
      </div>
    );
  }

  return (
    <div style={S.entryRow} onDoubleClick={onDoubleClick}>
      <span style={S.entryName}>{entry.name}</span>
      <span style={S.entrySep}>:</span>
      <span style={S.entryType}>{entry.returnType}</span>
      <button type="button" style={S.deleteBtn} onClick={onDelete}>×</button>
    </div>
  );
}

// ── DefSection (shared section wrapper) ──

interface DefSectionProps {
  title: string;
  sectionKind: "prop" | "signal" | "computed";
  entries: Array<DefEntry | ComputedEntry>;
  isAdding: boolean;
  onAddToggle: () => void;
  onAdd: (kind: "prop" | "signal" | "computed", name: string, type: string, extra?: string) => void;
  onCancelAdd: () => void;
  renderRow: (entry: DefEntry | ComputedEntry) => React.ReactNode;
  emptyText: string;
}

const DefSection = React.memo(function DefSection({
  title, sectionKind, entries, isAdding, onAddToggle,
  onAdd, onCancelAdd, renderRow, emptyText,
}: DefSectionProps) {
  return (
    <div style={S.section}>
      <div style={S.sectionHeader}>
        <span>{title}</span>
        <button type="button" style={S.addBtn} onClick={onAddToggle}>+</button>
      </div>
      {isAdding && (
        <AddForm kind={sectionKind} onAdd={onAdd} onCancel={onCancelAdd} />
      )}
      {entries.length === 0 && !isAdding && (
        <div style={S.emptyState}>{emptyText}</div>
      )}
      {entries.map((entry) => renderRow(entry))}
    </div>
  );
});

// ── Add form ──

interface AddFormProps {
  kind: "prop" | "signal" | "computed";
  onAdd: (kind: "prop" | "signal" | "computed", name: string, type: string, extra?: string) => void;
  onCancel: () => void;
}

function AddForm({ kind, onAdd, onCancel }: AddFormProps) {
  const nameRef = useRef<HTMLInputElement>(null);
  const typeRef = useRef<HTMLInputElement>(null);
  const extraRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    nameRef.current?.focus();
  }, []);

  const handleConfirm = () => {
    const name = nameRef.current?.value || "";
    const type = typeRef.current?.value || "String";
    const extra = extraRef.current?.value || "";
    onAdd(kind, name, type, extra);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter") handleConfirm();
    if (e.key === "Escape") onCancel();
  };

  return (
    <div style={S.addForm} onKeyDown={handleKeyDown}>
      <input ref={nameRef} placeholder="Name" aria-label="Add name" style={{ ...S.input, width: 72, fontWeight: "var(--ol-font-weight-600)" }} />
      <span style={S.entrySep}>:</span>
      <select ref={typeRef as unknown as React.RefObject<HTMLSelectElement>} defaultValue="String" aria-label="Add type"
        style={{ ...S.input, width: 96, cursor: "pointer" }}>
        {OMNI_TYPES.map((t) => <option key={t} value={t}>{t}</option>)}
      </select>
      {kind === "signal" && (
        <>
          <span style={S.entrySep}>=</span>
          <input ref={extraRef} placeholder="Default" aria-label="Default value" style={{ ...S.input, flex: 1 }} />
        </>
      )}
      {kind === "computed" && (
        <>
          <span style={S.entrySep}>=</span>
          <input ref={extraRef} placeholder="Expression" aria-label="Expression" style={{ ...S.input, flex: 1 }} />
        </>
      )}
      <button type="button" style={S.addFormConfirm} onClick={handleConfirm}>✓</button>
      <button type="button" style={S.addFormCancel} onClick={onCancel}>✕</button>
    </div>
  );
}

// ── 信号调试行（Task 9：数据信号）──

interface SignalDebugRowProps {
  ownerKey: string;
  usage: SignalUsage;
}

function SignalDebugRow({ ownerKey, usage }: SignalDebugRowProps) {
  const setDebugValue = usePoiesisStore((s) => s.setDebugValue);
  const debugValues = usePoiesisStore((s) => s.debugValues);
  const entry = debugValues[ownerKey]?.[usage.key];
  const text = entry?.text ?? "";

  const handleChange = (value: string) => {
    const parsed = parseDebugValue(usage.declaredType, value);
    setDebugValue(ownerKey, usage.key, value, parsed.valid);
  };

  const isNumeric = ["i32", "i64", "f64", "f32"].includes(usage.declaredType);
  const invalid = Boolean(entry && !entry.valid);

  return (
    <div style={S.debugRow}>
      <span style={S.debugName}>{usage.key}</span>
      <span style={S.debugType}>{usage.declaredType}</span>
      <input
        aria-label={`${usage.key} 值`}
        style={{ ...S.debugValueInput, ...(invalid ? S.invalidInput : {}) }}
        className={invalid ? "invalid" : undefined}
        value={text}
        placeholder="默认值"
        onChange={(e) => handleChange(e.target.value)}
      />
      {isNumeric && (
        <button type="button" style={S.stepBtn} onClick={() => {
          const cur = text === "" ? "0" : text;
          const next = String(Number(cur) + 1);
          handleChange(next);
        }}>＋</button>
      )}
    </div>
  );
}

// ── 界面信号卡（Task 10：按信号聚合 @if 分支胶囊 + @for 列表胶囊 + 值输入）──

interface InterfaceSignalCardProps {
  /** 调试值 ownerKey = 组件名（Task 9 语义；applyDebugOverrides 按此消费） */
  ownerKey: string;
  usage: SignalUsage;
  /** 组件 fn 节点源码（样本提取用；面板从 componentDefs.elements 收集） */
  fnSources: string[];
}

function InterfaceSignalCard({ ownerKey, usage, fnSources }: InterfaceSignalCardProps) {
  const setDebugValue = usePoiesisStore((s) => s.setDebugValue);
  const debugValues = usePoiesisStore((s) => s.debugValues);
  const debugOverridesEnabled = usePoiesisStore((s) => s.debugOverridesEnabled);
  const entry = debugValues[ownerKey]?.[usage.key];
  const text = entry?.text ?? "";
  const valid = entry ? entry.valid : true;

  // 输入框：解析后存字面量（String 自动补引号，与 Task 3 的 verbatim 覆盖契约一致）
  const applyValue = (raw: string) => {
    const parsed = parseDebugValue(usage.declaredType, raw);
    setDebugValue(ownerKey, usage.key, parsed.literal, parsed.valid);
  };

  // 分支胶囊：单信号简单比较可合成；复杂（比较不全/含调用）置灰。
  // 用 2D 结构而非 flatMap：brief 的 branchButtons[i*chainLen+j] 在链长不等时
  // 越位错配（测试契约之外的真实缺陷，见 task-10-report.md）。
  // 点选 → 合成字面量原样入 store（测试契约：String 不补引号，输入框回显原文）。
  const branchButtons = usage.conditions.map((chain) =>
    chain.branches.map((branch, branchIdx) => {
      const cmp = branch.comparisons[0];
      const synthesizable = cmp !== undefined && synthesizeConditionValue(cmp, "true").valid;
      const label = branch.isElse ? "else" : branch.exprText || "?";
      const handleClick = () => {
        if (!cmp || !debugOverridesEnabled) return;
        const result = synthesizeConditionValue(cmp, "true");
        if (!result.valid) return;
        setDebugValue(ownerKey, usage.key, result.literal, true);
      };
      return (
        <button key={branchIdx} type="button"
          style={S.branchChip(valid && entry !== undefined)}
          disabled={!synthesizable || !debugOverridesEnabled}
          title={!synthesizable ? "复杂条件，请在输入框手动调" : `驱动该分支成真`}
          onClick={handleClick}>
          {label}
        </button>
      );
    }),
  );

  // 列表胶囊：空数组 + 代码样本（struct 数组）+ 手动输入
  const samples = usage.loops.length > 0 && /Array</.test(usage.declaredType)
    ? extractAssignSamples(fnSources, usage.key.split(".").pop()!)
    : [];

  const badges = [
    usage.conditions.length > 0 ? `条件 ${usage.conditions.length}` : "",
    usage.loops.length > 0 ? `列表 ${usage.loops.length}` : "",
    usage.uses.interpolations > 0 ? `插值 ${usage.uses.interpolations}` : "",
    usage.uses.computedDeps > 0 ? `computed ${usage.uses.computedDeps}` : "",
  ].filter(Boolean).join(" · ");

  return (
    <div style={S.signalCard}>
      <div style={S.signalCardHeader}>
        <span style={S.debugName}>{usage.key}</span>
        <span style={S.debugType}>{usage.declaredType}</span>
        {badges && <span style={S.badgeText}>{badges}</span>}
      </div>

      {usage.conditions.length > 0 && (
        <div style={S.branchGroup}>
          {branchButtons.map((row, i) => (
            <div key={i} style={S.branchRow}>
              <span style={S.chainPrefix}>@if</span>
              {row.map((btn, j) => (
                <span key={j} style={S.branchChipWrap}>{btn}</span>
              ))}
            </div>
          ))}
        </div>
      )}

      {usage.loops.length > 0 && (
        <div style={S.loopGroup}>
          {usage.loops.map((loop, i) => (
            <div key={i} style={S.loopRow}>
              <span style={S.loopText}>@for in {loop.collectionText}</span>
              {loop.hasEmpty && <span style={S.loopEmptyTag}>@empty</span>}
              {loop.viaTransform && <span style={S.loopTransformTag}>经转换</span>}
            </div>
          ))}
          <div style={S.chipRow}>
            <button type="button" style={S.quickChip} disabled={!debugOverridesEnabled}
              onClick={() => applyValue("[]")}>空</button>
            {samples.map((s) => (
              <button key={s.label} type="button" style={S.quickChip} disabled={!debugOverridesEnabled}
                onClick={() => setDebugValue(ownerKey, usage.key, s.literal, true)}>
                {s.label}
              </button>
            ))}
          </div>
        </div>
      )}

      <div style={S.debugRow}>
        <span style={S.valueLabel}>值</span>
        <input
          aria-label={`${usage.key} 值`}
          style={{ ...S.debugValueInput, ...(entry && !entry.valid ? S.invalidInput : {}) }}
          className={entry && !entry.valid ? "invalid" : undefined}
          value={text}
          placeholder="默认值"
          onChange={(e) => applyValue(e.target.value)}
        />
        {!valid && <span style={S.invalidHint}>⚠ 格式不符</span>}
      </div>
    </div>
  );
}

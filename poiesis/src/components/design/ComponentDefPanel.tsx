import React, { useState, useCallback, useRef, useEffect } from "react";
import { usePoiesisStore } from "../../store";
import { useCanvasStore } from "../canvas/canvas-store";
import type { DefEntry, ComputedEntry } from "../../core/types/project-json";
import "./ComponentDefPanel.css";

/** Generate a simple PID (unique within component) */
function generatePid(): string {
  return `pid-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

export function ComponentDefPanel() {
  const activeTabId = usePoiesisStore((s) => s.designPage.activeTabId);
  const openTabs = usePoiesisStore((s) => s.designPage.openTabs);
  const componentDefs = usePoiesisStore((s) => s.componentDefs);
  const addComponentSignal = usePoiesisStore((s) => s.addComponentSignal);
  const removeComponentSignal = usePoiesisStore((s) => s.removeComponentSignal);
  const updateComponentSignal = usePoiesisStore((s) => s.updateComponentSignal);
  const addComponentProp = usePoiesisStore((s) => s.addComponentProp);
  const removeComponentProp = usePoiesisStore((s) => s.removeComponentProp);
  const updateComponentProp = usePoiesisStore((s) => s.updateComponentProp);
  const addComponentComputed = usePoiesisStore((s) => s.addComponentComputed);
  const removeComponentComputed = usePoiesisStore((s) => s.removeComponentComputed);
  const updateComponentComputed = usePoiesisStore((s) => s.updateComponentComputed);

  // Debug mode
  const [debugMode, setDebugMode] = useState(false);
  const debugSignalValues = usePoiesisStore((s) => s.debugSignalValues);
  const setDebugSignalValue = usePoiesisStore((s) => s.setDebugSignalValue);

  const panelPosition = useCanvasStore((s) => s.panelPosition);
  const isCollapsed = useCanvasStore((s) => s.isPanelCollapsed);
  const setPanelPosition = useCanvasStore((s) => s.setPanelPosition);
  const toggleCollapsed = useCanvasStore((s) => s.togglePanelCollapsed);

  // Drag state
  const dragRef = useRef<{ startX: number; startY: number; posX: number; posY: number } | null>(null);
  const [isAdding, setIsAdding] = useState<"prop" | "signal" | "computed" | null>(null);
  const [editingPid, setEditingPid] = useState<string | null>(null);

  // Get current component name
  const activeTab = openTabs.find((t) => t.id === activeTabId);
  const componentName = activeTab?.componentName || "main";
  const def = componentDefs[componentName];
  const signals = def?.signals || [];
  const props = def?.props || [];
  const computeds = def?.computeds || [];

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
  const renderPropsRow = useCallback((p: DefEntry) => (
    <DefEntryRow key={p.pid} entry={p} kind="prop"
      isEditing={editingPid === p.pid} onDoubleClick={() => handleDoubleClick(p.pid)}
      onEditSave={(f, v) => handleEditSave("prop", p.pid, f, v)}
      onDelete={() => handleDelete("prop", p.pid)} />
  ), [editingPid, handleDoubleClick, handleEditSave, handleDelete]);

  const renderSignalsRow = useCallback((s: DefEntry) => {
    const debugVal = debugSignalValues[componentName]?.[s.name];
    return (
      <DefEntryRow key={s.pid} entry={s} kind="signal"
        isEditing={editingPid === s.pid}
        onDoubleClick={() => handleDoubleClick(s.pid)}
        onEditSave={(f, v) => handleEditSave("signal", s.pid, f, v)}
        onDelete={() => handleDelete("signal", s.pid)}
        debugMode={debugMode}
        debugValue={debugVal as string | undefined}
        onDebugValueChange={(v) => setDebugSignalValue(componentName, s.name, v || undefined)}
      />
    );
  }, [editingPid, handleDoubleClick, handleEditSave, handleDelete, debugMode, debugSignalValues, componentName, setDebugSignalValue]);

  const renderComputedsRow = useCallback((c: ComputedEntry) => (
    <ComputedEntryRow key={c.pid} entry={c}
      isEditing={editingPid === c.pid} onDoubleClick={() => handleDoubleClick(c.pid)}
      onEditSave={(f, v) => handleEditSave("computed", c.pid, f, v)}
      onDelete={() => handleDelete("computed", c.pid)} />
  ), [editingPid, handleDoubleClick, handleEditSave, handleDelete]);

  return (
    <div
      className="component-def-panel"
      style={{
        left: panelPosition.x,
        top: panelPosition.y,
      }}
      onPointerDown={(e) => e.stopPropagation()}
    >
      {/* Header */}
      <div
        className="panel-header"
        onPointerDown={handlePointerDown}
        onPointerMove={handlePointerMove}
        onPointerUp={handlePointerUp}
      >
        <span className="panel-header-title">{componentName}</span>
        <button type="button"
          className={`debug-toggle-btn${debugMode ? " active" : ""}`}
          onClick={() => {
            if (debugMode) {
              // Clear debug values when turning off debug mode
              for (const sig of signals) {
                setDebugSignalValue(componentName, sig.name, undefined);
              }
            }
            setDebugMode((v) => !v);
          }}
          title={debugMode ? "Debug mode ON — signal edits are preview-only" : "Enable debug mode to preview signal changes"}
        >
          {debugMode ? "🔧 ON" : "🔧"}
        </button>
        <button type="button" className="panel-collapse-btn" onClick={toggleCollapsed}>
          {isCollapsed ? "□" : "－"}
        </button>
      </div>

      {!isCollapsed && (
        <div className="panel-body">
          <DefSection
            title="Props" sectionKind="prop"
            entries={props} isAdding={isAdding === "prop"}
            onAddToggle={() => setIsAdding(isAdding === "prop" ? null : "prop")}
            onAdd={handleAdd} onCancelAdd={() => setIsAdding(null)}
            editingPid={editingPid} onDoubleClick={handleDoubleClick}
            onEditSave={handleEditSave} onDelete={handleDelete}
            renderRow={renderPropsRow}
            emptyText="No props defined"
          />

          <DefSection
            title="Signals" sectionKind="signal"
            entries={signals} isAdding={isAdding === "signal"}
            onAddToggle={() => setIsAdding(isAdding === "signal" ? null : "signal")}
            onAdd={handleAdd} onCancelAdd={() => setIsAdding(null)}
            editingPid={editingPid} onDoubleClick={handleDoubleClick}
            onEditSave={handleEditSave} onDelete={handleDelete}
            renderRow={renderSignalsRow}
            emptyText="No signals defined"
          />

          <DefSection
            title="Computed" sectionKind="computed"
            entries={computeds} isAdding={isAdding === "computed"}
            onAddToggle={() => setIsAdding(isAdding === "computed" ? null : "computed")}
            onAdd={handleAdd} onCancelAdd={() => setIsAdding(null)}
            editingPid={editingPid} onDoubleClick={handleDoubleClick}
            onEditSave={handleEditSave} onDelete={handleDelete}
            renderRow={renderComputedsRow}
            emptyText="No computeds defined"
          />
        </div>
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
  debugMode?: boolean;
  debugValue?: string;
  onDebugValueChange?: (value: string) => void;
}

function DefEntryRow({ entry, kind, isEditing, onDoubleClick, onEditSave, onDelete, debugMode, debugValue, onDebugValueChange }: DefEntryRowProps) {
  if (isEditing) {
    return (
      <div className="entry-row">
        <input
          aria-label="Entry name"
          className="entry-name-input"
          defaultValue={entry.name}
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span>:</span>
        <input
          aria-label="Entry type"
          className="entry-type-input"
          defaultValue={entry.type}
          onBlur={(e) => onEditSave("type", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("type", e.currentTarget.value); }}
        />
        {kind === "signal" && (
          <>
            <span>=</span>
            <input
              aria-label="Default value"
              className="entry-default-input"
              defaultValue={String(entry.defaultValue ?? "")}
              placeholder="Default"
              onBlur={(e) => onEditSave("defaultValue", e.target.value)}
              onKeyDown={(e) => { if (e.key === "Enter") onEditSave("defaultValue", e.currentTarget.value); }}
            />
          </>
        )}
      </div>
    );
  }

  // Display mode — show debug input for signals when debugMode is on
  const showDebugInput = kind === "signal" && debugMode;

  return (
    <div className={`entry-row${showDebugInput ? " debug-row" : ""}`} onDoubleClick={onDoubleClick}>
      <span className="entry-name">{entry.name}</span>
      <span>:</span>
      <span className="entry-type">{entry.type}</span>
      <span>=</span>
      {showDebugInput ? (
        <input
          className="debug-value-input"
          value={debugValue ?? ""}
          placeholder={String(entry.defaultValue ?? "")}
          onChange={(e) => onDebugValueChange?.(e.target.value)}
          onClick={(e) => e.stopPropagation()}
        />
      ) : (
        <span className="entry-default">{String(entry.defaultValue ?? "")}</span>
      )}
      <button type="button" className="entry-delete-btn" onClick={onDelete}>{String.fromCharCode(0x2715)}</button>
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
      <div className="entry-row">
        <input
          aria-label="Computed name"
          className="entry-name-input"
          defaultValue={entry.name}
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span>:</span>
        <input
          aria-label="Computed return type"
          className="entry-type-input"
          defaultValue={entry.returnType}
          onBlur={(e) => onEditSave("returnType", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("returnType", e.currentTarget.value); }}
        />
      </div>
    );
  }

  return (
    <div className="entry-row" onDoubleClick={onDoubleClick}>
      <span className="entry-name">{entry.name}</span>
      <span>:</span>
      <span className="entry-type">{entry.returnType}</span>
      <button type="button" className="entry-delete-btn" onClick={onDelete}>{String.fromCharCode(0x2715)}</button>
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
  editingPid: string | null;
  onDoubleClick: (pid: string) => void;
  onEditSave: (kind: "prop" | "signal" | "computed", pid: string, field: string, value: string) => void;
  onDelete: (kind: "prop" | "signal" | "computed", pid: string) => void;
  renderRow: (entry: DefEntry | ComputedEntry) => React.ReactNode;
  emptyText: string;
}

const DefSection = React.memo(function DefSection({
  title, sectionKind, entries, isAdding, onAddToggle,
  onAdd, onCancelAdd, editingPid, onDoubleClick,
  onEditSave, onDelete, renderRow, emptyText,
}: DefSectionProps) {
  return (
    <div className="panel-section">
      <div className="section-header">
        <span>{title}</span>
        <button type="button" className="section-add-btn" onClick={onAddToggle}>+</button>
      </div>
      {isAdding && (
        <AddForm kind={sectionKind} onAdd={onAdd} onCancel={onCancelAdd} />
      )}
      {entries.length === 0 && !isAdding && (
        <div className="empty-state">{emptyText}</div>
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
    <div className="add-form" onKeyDown={handleKeyDown}>
      <input ref={nameRef} placeholder="Name" aria-label="Add name" className="entry-name-input" />
      <span>:</span>
      <input ref={typeRef} placeholder="Type" aria-label="Add type" className="entry-type-input" defaultValue="String" />
      {kind === "signal" && (
        <>
          <span>=</span>
          <input ref={extraRef} placeholder="Default" aria-label="Default value" className="entry-default-input" />
        </>
      )}
      {kind === "computed" && (
        <>
          <span>=</span>
          <input ref={extraRef} placeholder="Expression" aria-label="Expression" className="entry-default-input" />
        </>
      )}
      <button type="button" className="add-form-confirm" onClick={handleConfirm}>{String.fromCharCode(0x2713)}</button>
      <button type="button" className="add-form-cancel" onClick={onCancel}>{String.fromCharCode(0x2715)}</button>
    </div>
  );
}

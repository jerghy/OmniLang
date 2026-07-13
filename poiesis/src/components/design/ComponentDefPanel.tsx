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
  const handleDoubleClick = (pid: string) => {
    setEditingPid(pid);
  };

  const handleEditSave = (kind: "prop" | "signal" | "computed", pid: string, field: string, value: string) => {
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
  };

  // Delete entry
  const handleDelete = (kind: "prop" | "signal" | "computed", pid: string) => {
    if (kind === "prop") removeComponentProp(componentName, pid);
    else if (kind === "signal") removeComponentSignal(componentName, pid);
    else removeComponentComputed(componentName, pid);
  };

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
        <button className="panel-collapse-btn" onClick={toggleCollapsed}>
          {isCollapsed ? "□" : "－"}
        </button>
      </div>

      {!isCollapsed && (
        <div className="panel-body">
          {/* Props section */}
          <div className="panel-section">
            <div className="section-header">
              <span>Props</span>
              <button className="section-add-btn" onClick={() => setIsAdding(isAdding === "prop" ? null : "prop")}>+</button>
            </div>
            {isAdding === "prop" && (
              <AddForm kind="prop" onAdd={handleAdd} onCancel={() => setIsAdding(null)} />
            )}
            {props.length === 0 && !isAdding && (
              <div className="empty-state">No props defined</div>
            )}
            {props.map((p) => (
              <DefEntryRow
                key={p.pid}
                entry={p}
                kind="prop"
                isEditing={editingPid === p.pid}
                onDoubleClick={() => handleDoubleClick(p.pid)}
                onEditSave={(field, value) => handleEditSave("prop", p.pid, field, value)}
                onDelete={() => handleDelete("prop", p.pid)}
              />
            ))}
          </div>

          {/* Signals section */}
          <div className="panel-section">
            <div className="section-header">
              <span>Signals</span>
              <button className="section-add-btn" onClick={() => setIsAdding(isAdding === "signal" ? null : "signal")}>+</button>
            </div>
            {isAdding === "signal" && (
              <AddForm kind="signal" onAdd={handleAdd} onCancel={() => setIsAdding(null)} />
            )}
            {signals.length === 0 && !isAdding && (
              <div className="empty-state">No signals defined</div>
            )}
            {signals.map((s) => (
              <DefEntryRow
                key={s.pid}
                entry={s}
                kind="signal"
                isEditing={editingPid === s.pid}
                onDoubleClick={() => handleDoubleClick(s.pid)}
                onEditSave={(field, value) => handleEditSave("signal", s.pid, field, value)}
                onDelete={() => handleDelete("signal", s.pid)}
              />
            ))}
          </div>

          {/* Computed section */}
          <div className="panel-section">
            <div className="section-header">
              <span>Computed</span>
              <button className="section-add-btn" onClick={() => setIsAdding(isAdding === "computed" ? null : "computed")}>+</button>
            </div>
            {isAdding === "computed" && (
              <AddForm kind="computed" onAdd={handleAdd} onCancel={() => setIsAdding(null)} />
            )}
            {computeds.length === 0 && !isAdding && (
              <div className="empty-state">No computeds defined</div>
            )}
            {computeds.map((c) => (
              <ComputedEntryRow
                key={c.pid}
                entry={c}
                isEditing={editingPid === c.pid}
                onDoubleClick={() => handleDoubleClick(c.pid)}
                onEditSave={(field, value) => handleEditSave("computed", c.pid, field, value)}
                onDelete={() => handleDelete("computed", c.pid)}
              />
            ))}
          </div>
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
}

function DefEntryRow({ entry, kind, isEditing, onDoubleClick, onEditSave, onDelete }: DefEntryRowProps) {
  if (isEditing) {
    return (
      <div className="entry-row">
        <input
          className="entry-name-input"
          defaultValue={entry.name}
          autoFocus
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span>:</span>
        <input
          className="entry-type-input"
          defaultValue={entry.type}
          onBlur={(e) => onEditSave("type", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("type", e.currentTarget.value); }}
        />
        {kind === "signal" && (
          <>
            <span>=</span>
            <input
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

  return (
    <div className="entry-row" onDoubleClick={onDoubleClick}>
      <span className="entry-name">{entry.name}</span>
      <span>:</span>
      <span className="entry-type">{entry.type}</span>
      {kind === "signal" && entry.defaultValue !== undefined && (
        <>
          <span>=</span>
          <span className="entry-default">{String(entry.defaultValue)}</span>
        </>
      )}
      <button className="entry-delete-btn" onClick={onDelete}>{String.fromCharCode(0x2715)}</button>
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
          className="entry-name-input"
          defaultValue={entry.name}
          autoFocus
          onBlur={(e) => onEditSave("name", e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") onEditSave("name", e.currentTarget.value); }}
        />
        <span>:</span>
        <input
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
      <button className="entry-delete-btn" onClick={onDelete}>{String.fromCharCode(0x2715)}</button>
    </div>
  );
}

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
      <input ref={nameRef} placeholder="Name" className="entry-name-input" />
      <span>:</span>
      <input ref={typeRef} placeholder="Type" className="entry-type-input" defaultValue="String" />
      {kind === "signal" && (
        <>
          <span>=</span>
          <input ref={extraRef} placeholder="Default" className="entry-default-input" />
        </>
      )}
      {kind === "computed" && (
        <>
          <span>=</span>
          <input ref={extraRef} placeholder="Expression" className="entry-default-input" />
        </>
      )}
      <button className="add-form-confirm" onClick={handleConfirm}>{String.fromCharCode(0x2713)}</button>
      <button className="add-form-cancel" onClick={onCancel}>{String.fromCharCode(0x2715)}</button>
    </div>
  );
}

// ui/panels/StyleDesignModal.tsx
// Modal dialog for editing component-level CSS styles (variables + classes).

import { useState } from "react";
import { useStyleRegistryStore } from "../../core/state/styleRegistry";
import { useUndoStore } from "../../store/undo";
import { uid } from "../../core/utils/id";
import { CssVariableEditor } from "./CssVariableEditor";
import { ClassCard } from "./ClassCard";
import { AddCssVariable } from "../../core/commands/AddCssVariable";
import { RemoveCssVariable } from "../../core/commands/RemoveCssVariable";
import { UpdateCssVariable } from "../../core/commands/UpdateCssVariable";
import { UpdateStyleClass } from "../../core/commands/UpdateStyleClass";
import { AddStyleClass } from "../../core/commands/AddStyleClass";
import { RemoveStyleClass } from "../../core/commands/RemoveStyleClass";
import type { CommandContext } from "../../core/commands/types";
import type { CssVariable, StyleClass } from "../../core/types/style";

interface Props {
  componentName: string;
  ctx: CommandContext;
  onClose: () => void;
}

export function StyleDesignModal({ componentName, ctx, onClose }: Props) {
  const registry = useStyleRegistryStore((s) => s.getRegistry(componentName));
  const undoExec = useUndoStore((s) => s.execute);

  const [varsExpanded, setVarsExpanded] = useState(true);
  const [classesExpanded, setClassesExpanded] = useState(true);

  const handleAddVariable = () => {
    const v: CssVariable = { id: uid("v"), name: "--unnamed", segments: [{ type: "text", value: "" }] };
    undoExec(new AddCssVariable(componentName, v), ctx);
  };

  const handleRemoveVariable = (id: string) => {
    const v = registry.variables.find((x) => x.id === id);
    if (v) undoExec(new RemoveCssVariable(componentName, v), ctx);
  };

  const handleChangeVariable = (id: string, patch: Partial<CssVariable>) => {
    const v = registry.variables.find((x) => x.id === id);
    if (!v) return;
    const oldName = v.name;
    const newName = patch.name ?? oldName;
    const oldSegments = v.segments;
    const newSegments = patch.segments ?? oldSegments;
    if (oldName !== newName || oldSegments !== newSegments) {
      undoExec(new UpdateCssVariable(componentName, id, { name: oldName, segments: oldSegments }, patch), ctx);
    }
  };

  const handleAddClass = () => {
    const c: StyleClass = { id: uid("c"), name: "", properties: [] };
    undoExec(new AddStyleClass(componentName, c), ctx);
  };

  const handleRemoveClass = (id: string) => {
    const c = registry.classes.find((x) => x.id === id);
    if (c) undoExec(new RemoveStyleClass(componentName, c), ctx);
  };

  const handleChangeClass = (updated: StyleClass) => {
    const existing = registry.classes.find((c) => c.id === updated.id);
    if (existing) {
      undoExec(new UpdateStyleClass(componentName, existing, updated), ctx);
    }
  };

  return (
    <>
      {/* Backdrop */}
      <div
        onClick={onClose}
        style={{ position: "fixed", inset: 0, zIndex: 9998, background: "rgba(0,0,0,0.3)" }}
      />

      {/* Modal */}
      <div style={{
        position: "fixed", top: "50%", left: "50%", transform: "translate(-50%, -50%)",
        zIndex: 9999, background: "#fff", borderRadius: 12,
        boxShadow: "0 8px 40px rgba(0,0,0,0.2)",
        width: 900, maxWidth: "95vw", maxHeight: "90vh",
        overflow: "auto", padding: "24px",
      }}>
        <h2 style={{ margin: "0 0 16px", fontSize: 16, fontWeight: 600 }}>CSS 样式设计</h2>

        {/* CSS Variables section */}
        <div style={{ marginBottom: 16 }}>
          <div
            onClick={() => setVarsExpanded(!varsExpanded)}
            style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer", marginBottom: 8 }}
          >
            <span style={{ fontSize: 11 }}>{varsExpanded ? "▼" : "▶"}</span>
            <span style={{ fontSize: 13, fontWeight: 600, color: "#374151" }}>CSS 变量</span>
            <span style={{ marginLeft: "auto", fontSize: 11, color: "#4F46E5", cursor: "pointer" }}
              onClick={(e) => { e.stopPropagation(); handleAddVariable(); }}>
              + 添加变量
            </span>
          </div>
          {varsExpanded && (
            <CssVariableEditor
              variables={registry.variables}
              onAdd={handleAddVariable}
              onRemove={handleRemoveVariable}
              onChange={handleChangeVariable}
            />
          )}
        </div>

        {/* Class definitions section */}
        <div>
          <div
            onClick={() => setClassesExpanded(!classesExpanded)}
            style={{ display: "flex", alignItems: "center", gap: 4, cursor: "pointer", marginBottom: 8 }}
          >
            <span style={{ fontSize: 11 }}>{classesExpanded ? "▼" : "▶"}</span>
            <span style={{ fontSize: 13, fontWeight: 600, color: "#374151" }}>类定义</span>
            <span style={{ marginLeft: "auto", fontSize: 11, color: "#4F46E5", cursor: "pointer" }}
              onClick={(e) => { e.stopPropagation(); handleAddClass(); }}>
              + 新建 Class
            </span>
          </div>
          {classesExpanded && (
            <div>
              {registry.classes.map((c) => (
                <ClassCard
                  key={c.id}
                  styleClass={c}
                  onChange={handleChangeClass}
                  onDelete={() => handleRemoveClass(c.id)}
                />
              ))}
              {registry.classes.length === 0 && (
                <div style={{ padding: 16, color: "#9CA3AF", fontSize: 12, textAlign: "center" }}>
                  暂无类定义，点击 "+ 新建 Class" 添加
                </div>
              )}
            </div>
          )}
        </div>

        {/* Footer */}
        <div style={{ display: "flex", justifyContent: "flex-end", gap: 8, marginTop: 20, paddingTop: 16, borderTop: "1px solid #e5e7eb" }}>
          <button onClick={onClose} style={{
            padding: "8px 20px", fontSize: 13, border: "1px solid #d1d5db",
            borderRadius: 6, background: "#fff", cursor: "pointer",
          }}>
            取消
          </button>
          <button onClick={onClose} style={{
            padding: "8px 20px", fontSize: 13, border: "none", borderRadius: 6,
            background: "#4F46E5", color: "#fff", cursor: "pointer", fontWeight: 600,
          }}>
            保存
          </button>
        </div>
      </div>
    </>
  );
}

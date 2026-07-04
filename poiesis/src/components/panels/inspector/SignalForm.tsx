// components/panels/inspector/SignalForm.tsx
// Signal/prop/event definition form for component-level metadata.

import { useState, useCallback } from "react";

interface SignalDefinition {
  name: string;
  type: string;
  defaultValue: string;
}

interface SignalFormProps {
  title: string;
  signals: SignalDefinition[];
  onAdd: (signal: SignalDefinition) => void;
  onRemove: (index: number) => void;
  onChange: (index: number, signal: SignalDefinition) => void;
}

const SIGNAL_TYPES = [
  "string",
  "number",
  "boolean",
  "any",
  "void",
  "Signal<string>",
  "Signal<number>",
  "Signal<boolean>",
];

export function SignalForm({
  title,
  signals,
  onAdd,
  onRemove,
  onChange,
}: SignalFormProps) {
  const [newName, setNewName] = useState("");
  const [newType, setNewType] = useState("string");
  const [newDefault, setNewDefault] = useState("");

  const handleAdd = useCallback(() => {
    if (!newName.trim()) return;
    onAdd({
      name: newName.trim(),
      type: newType,
      defaultValue: newDefault,
    });
    setNewName("");
    setNewDefault("");
  }, [newName, newType, newDefault, onAdd]);

  return (
    <div className="signal-form" style={{ marginBottom: 12 }}>
      <div
        style={{
          fontSize: 11,
          fontWeight: 600,
          color: "#6b7280",
          textTransform: "uppercase",
          marginBottom: 6,
        }}
      >
        {title}
      </div>

      {/* Existing signals */}
      {signals.length === 0 && (
        <div style={{ fontSize: 11, color: "#94a3b8", marginBottom: 6 }}>
          暂无定义
        </div>
      )}

      {signals.map((sig, i) => (
        <div
          key={i}
          style={{
            display: "flex",
            alignItems: "center",
            gap: 4,
            marginBottom: 4,
            padding: "4px 6px",
            background: "#f8fafc",
            borderRadius: 4,
          }}
        >
          <input
            type="text"
            value={sig.name}
            onChange={(e) =>
              onChange(i, { ...sig, name: e.target.value })
            }
            style={{
              width: 80,
              padding: "2px 4px",
              fontSize: 11,
              border: "1px solid #e2e8f0",
              borderRadius: 3,
              outline: "none",
              fontFamily: "monospace",
            }}
          />
          <select
            value={sig.type}
            onChange={(e) =>
              onChange(i, { ...sig, type: e.target.value })
            }
            style={{
              padding: "2px 4px",
              fontSize: 10,
              border: "1px solid #e2e8f0",
              borderRadius: 3,
              outline: "none",
              background: "#fff",
            }}
          >
            {SIGNAL_TYPES.map((t) => (
              <option key={t} value={t}>
                {t}
              </option>
            ))}
          </select>
          <input
            type="text"
            value={sig.defaultValue}
            onChange={(e) =>
              onChange(i, { ...sig, defaultValue: e.target.value })
            }
            placeholder="默认值"
            style={{
              width: 60,
              padding: "2px 4px",
              fontSize: 11,
              border: "1px solid #e2e8f0",
              borderRadius: 3,
              outline: "none",
            }}
          />
          <button
            onClick={() => onRemove(i)}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              color: "#ef4444",
              fontSize: 12,
              padding: "2px 4px",
            }}
          >
            ×
          </button>
        </div>
      ))}

      {/* Add new signal form */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 4,
          marginTop: 4,
        }}
      >
        <input
          type="text"
          value={newName}
          onChange={(e) => setNewName(e.target.value)}
          placeholder="名称"
          style={{
            width: 80,
            padding: "3px 6px",
            fontSize: 11,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "none",
          }}
          onKeyDown={(e) => {
            if (e.key === "Enter") handleAdd();
          }}
        />
        <select
          value={newType}
          onChange={(e) => setNewType(e.target.value)}
          style={{
            padding: "3px 4px",
            fontSize: 10,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "none",
            background: "#fff",
          }}
        >
          {SIGNAL_TYPES.map((t) => (
            <option key={t} value={t}>
              {t}
            </option>
          ))}
        </select>
        <input
          type="text"
          value={newDefault}
          onChange={(e) => setNewDefault(e.target.value)}
          placeholder="默认值"
          style={{
            width: 60,
            padding: "3px 6px",
            fontSize: 11,
            border: "1px solid #e2e8f0",
            borderRadius: 4,
            outline: "none",
          }}
        />
        <button
          onClick={handleAdd}
          disabled={!newName.trim()}
          style={{
            padding: "3px 8px",
            fontSize: 11,
            border: "none",
            borderRadius: 4,
            background: newName.trim() ? "#6366F1" : "#e2e8f0",
            color: newName.trim() ? "#fff" : "#94a3b8",
            cursor: newName.trim() ? "pointer" : "default",
            fontWeight: 600,
          }}
        >
          +
        </button>
      </div>
    </div>
  );
}

// poiesis/src/components/canvas/code/CodeCardEditor.tsx
import React, { useState, useCallback } from "react";
import type { CodeOp } from "../../../core/types/code-op";
import { CardList } from "./cards/CardList";
import { AutocompleteMenu, getAutocompleteItems, type AutocompleteItem } from "./cards/AutocompleteMenu";
import "./cards"; // side-effect: registers all card components
import { uid } from "../../../core/utils/id";

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

export const CodeCardEditor: React.FC<Props> = ({
  elementId, initialOps, onSave, onClose,
}) => {
  const [ops, setOps] = useState<CodeOp[]>(initialOps);
  const [selectedPid, setSelectedPid] = useState<string | null>(
    initialOps.length > 0 ? initialOps[0].pid : null
  );
  const [showAutocomplete, setShowAutocomplete] = useState(false);
  const [autocompleteIndex, setAutocompleteIndex] = useState(0);
  const [autocompleteItems, setAutocompleteItems] = useState<AutocompleteItem[]>([]);
  const [menuPos, setMenuPos] = useState({ top: 100, left: 50 });
  const [pendingInsertIndex, setPendingInsertIndex] = useState<number>(ops.length);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    // Close on Escape
    if (e.key === "Escape") {
      if (showAutocomplete) {
        setShowAutocomplete(false);
      } else {
        onClose();
      }
      return;
    }

    // Navigate autocomplete
    if (showAutocomplete) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setAutocompleteIndex(i => Math.min(i + 1, autocompleteItems.length - 1));
        return;
      }
      if (e.key === "ArrowUp") {
        e.preventDefault();
        setAutocompleteIndex(i => Math.max(i - 1, 0));
        return;
      }
      if (e.key === "Enter") {
        e.preventDefault();
        const item = autocompleteItems[autocompleteIndex];
        if (item) insertOp(item, pendingInsertIndex);
        setShowAutocomplete(false);
        return;
      }
    }

    // Regular typing triggers autocomplete
    if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
      setMenuPos({ top: 200 + pendingInsertIndex * 32, left: 60 });
      setAutocompleteItems(getAutocompleteItems(""));
      setAutocompleteIndex(0);
      setShowAutocomplete(true);
    }
  }, [showAutocomplete, autocompleteIndex, autocompleteItems, pendingInsertIndex]);

  const insertOp = (item: AutocompleteItem, atIndex: number) => {
    const pid = uid("op");
    let newOp: CodeOp;

    switch (item.kind) {
      case "declare_var":
        newOp = { kind: "declare_var", pid, mutable: false, name: "", scope: "fn" };
        break;
      case "declare_var_mut":
        newOp = { kind: "declare_var", pid, mutable: true, name: "", scope: "fn" };
        break;
      case "set_var":
        newOp = { kind: "set_var", pid, name: "", operator: "=", value: "" };
        break;
      case "if_block":
        newOp = { kind: "if_block", pid, branches: [{ condition: "", body: [] }] };
        break;
      case "while":
        newOp = { kind: "while", pid, condition: "", body: [] };
        break;
      case "for_in":
        newOp = { kind: "for_in", pid, itemName: "", collection: "", body: [] };
        break;
      case "return":
        newOp = { kind: "return", pid };
        break;
      case "await":
        newOp = { kind: "await", pid, value: "" };
        break;
      case "break":
        newOp = { kind: "break", pid };
        break;
      case "continue":
        newOp = { kind: "continue", pid };
        break;
      case "comment":
        newOp = { kind: "comment", pid, text: "" };
        break;
      default:
        newOp = { kind: "raw", pid, source: "" };
    }

    const newOps = [...ops];
    newOps.splice(atIndex, 0, newOp);
    setOps(newOps);
    setSelectedPid(pid);
  };

  const handleSave = () => {
    // Filter out empty raw ops before saving
    const cleanOps = ops.filter(op => {
      if (op.kind === "raw" && !op.source.trim()) return false;
      if (op.kind === "declare_var" && !op.name) return false;
      return true;
    });
    onSave(cleanOps);
  };

  return (
    <div className="code-card-editor-overlay" style={{
      position: "fixed", inset: 0, zIndex: 9999,
      background: "rgba(0,0,0,0.3)", display: "flex",
      alignItems: "center", justifyContent: "center",
    }}>
      <div className="code-card-editor" tabIndex={0} onKeyDown={handleKeyDown}
        style={{
          width: "80vw", height: "80vh", background: "#fff",
          borderRadius: 12, boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          display: "flex", flexDirection: "column", overflow: "hidden",
        }}>
        {/* Header */}
        <div className="editor-header" style={{
          display: "flex", justifyContent: "space-between",
          alignItems: "center", padding: "12px 16px",
          borderBottom: "1px solid #e2e8f0",
        }}>
          <h3 style={{ margin: 0, fontSize: 14, fontWeight: 600 }}>
            Code Card Editor — {ops.length} ops
          </h3>
          <div style={{ display: "flex", gap: 8 }}>
            <button onClick={handleSave}
              style={{
                padding: "6px 16px", background: "#6366f1", color: "#fff",
                border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13,
              }}>
              Save
            </button>
            <button onClick={onClose}
              style={{
                padding: "6px 16px", background: "#e2e8f0", color: "#475569",
                border: "none", borderRadius: 6, cursor: "pointer", fontSize: 13,
              }}>
              Cancel
            </button>
          </div>
        </div>

        {/* Card list area */}
        <div className="editor-body" style={{ flex: 1, overflow: "auto", padding: 16, position: "relative" }}>
          <CardList
            ops={ops}
            selectedPid={selectedPid}
            onSelect={setSelectedPid}
            onUpdate={(idx, updated) => {
              const newOps = [...ops];
              newOps[idx] = updated;
              setOps(newOps);
            }}
            onDelete={(idx) => {
              setOps(ops.filter((_, i) => i !== idx));
            }}
            onInsertAfter={(idx) => {
              setPendingInsertIndex(idx + 1);
              setShowAutocomplete(true);
              setAutocompleteItems(getAutocompleteItems(""));
              setAutocompleteIndex(0);
              setMenuPos({ top: 200 + (idx + 1) * 32, left: 60 });
            }}
          />

          {showAutocomplete && (
            <AutocompleteMenu
              items={autocompleteItems}
              selectedIndex={autocompleteIndex}
              onSelect={(item) => {
                insertOp(item, pendingInsertIndex);
                setShowAutocomplete(false);
              }}
              onClose={() => setShowAutocomplete(false)}
              position={menuPos}
            />
          )}
        </div>
      </div>
    </div>
  );
};

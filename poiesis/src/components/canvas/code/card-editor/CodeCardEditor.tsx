// poiesis/src/components/canvas/code/card-editor/CodeCardEditor.tsx
import { useState, useCallback } from "react";
import type { CodeOp } from "../../../../core/types/code-op";
import { opsToRows, rowsToOps, type EditorRow } from "./row-utils";
import { RowList } from "./RowList";
import { AutocompleteMenu } from "./AutocompleteMenu";
import {
  getAutocompleteItems,
  detectCardKindFromItem,
  type AutocompleteItem,
} from "./autocomplete-rules";
import { uid } from "../../../../core/utils/id";
import { emitOps } from "../../../../services/codegen/code-op-emitter";
import { parseOpsWasm } from "../../../../services/codegen/code-op-parser";
import { CodeEditorModal } from "../CodeEditorModal";
import styles from "./CodeCardEditor.module.css";

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

export const CodeCardEditor: React.FC<Props> = ({
  elementId,
  initialOps,
  initialSource,
  language = "omnilang",
  onSave,
  onClose,
}) => {
  const [rows, setRows] = useState<EditorRow[]>(() => opsToRows(initialOps));
  const [focusedIndex, setFocusedIndex] = useState(0);
  const [showMonaco, setShowMonaco] = useState(false);

  // autocomplete state
  const [acShow, setAcShow] = useState(false);
  const [acItems, setAcItems] = useState<AutocompleteItem[]>([]);
  const [acIndex, setAcIndex] = useState(0);
  const [acRowIndex, setAcRowIndex] = useState(0);

  // ── Row mutation helpers ──

  const replaceRow = useCallback(
    (idx: number, newRow: EditorRow) => {
      setRows((prev) => {
        const next = [...prev];
        next[idx] = newRow;
        return next;
      });
    },
    [],
  );

  const insertRow = useCallback(
    (idx: number, row: EditorRow) => {
      setRows((prev) => {
        const next = [...prev];
        next.splice(idx + 1, 0, row);
        return next;
      });
    },
    [],
  );

  const deleteRow = useCallback(
    (idx: number) => {
      setRows((prev) => {
        if (prev.length <= 1) return prev;
        const next = [...prev];
        next.splice(idx, 1);
        return next;
      });
    },
    [],
  );

  // ── Autocomplete ──

  const triggerAutocomplete = useCallback(
    (rowIdx: number, token: string) => {
      const items = getAutocompleteItems(token, { parentKind: null, slotType: null });
      if (items.length > 0) {
        setAcItems(items);
        setAcIndex(0);
        setAcRowIndex(rowIdx);
        setAcShow(true);
      } else {
        setAcShow(false);
      }
    },
    [],
  );

  const executeAutocomplete = useCallback(
    (item: AutocompleteItem) => {
      setAcShow(false);
      const cardKind = detectCardKindFromItem(item);
      if (!cardKind) return;

      const pid = uid("op");
      let newOp: CodeOp;

      switch (cardKind) {
        case "declare_var":
          newOp = {
            kind: "declare_var",
            pid,
            mutable: item.kind === "declare_var_mut",
            name: "",
            scope: "fn",
          };
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

      // Replace the input row with rows from the new op
      const newRows = opsToRows([newOp]);
      // remove trailing input added by opsToRows, we replace current row
      if (newRows.length > 1 && newRows[newRows.length - 1].kind === "input") {
        newRows.pop();
      }

      setRows((prev) => {
        const next = [...prev];
        next.splice(acRowIndex, 1, ...newRows);
        return next;
      });
      setFocusedIndex(acRowIndex);
    },
    [acRowIndex],
  );

  // ── Card slot updates ──

  const updateCardOp = useCallback(
    (idx: number, updatedOp: CodeOp) => {
      replaceRow(idx, { kind: "card", op: updatedOp, indent: rows[idx].kind === "card" ? rows[idx].indent : 0 });
    },
    [rows, replaceRow],
  );

  // ── Save ──

  const handleSave = () => {
    const ops = rowsToOps(rows);
    onSave(ops);
  };

  // ── Monaco fallback ──

  if (showMonaco) {
    return (
      <CodeEditorModal
        elementId={elementId}
        initialSource={initialSource || emitOps(rowsToOps(rows))}
        initialDescription=""
        language={language}
        onSave={async (source, _desc) => {
          try {
            const parsed = await parseOpsWasm(source);
            onSave(parsed);
          } catch {
            onSave([{ kind: "raw", pid: uid("op"), source }]);
          }
        }}
        onClose={() => setShowMonaco(false)}
      />
    );
  }

  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        <div className={styles.header}>
          <h3 className={styles.title}>
            Code Card Editor — {rows.filter((r) => r.kind === "card").length} cards
          </h3>
          <div className={styles.actions}>
            <button type="button" className={styles.textModeBtn} onClick={() => setShowMonaco(true)}>
              文本模式
            </button>
            <button type="button" className={styles.saveBtn} onClick={handleSave}>
              Save
            </button>
            <button type="button" className={styles.cancelBtn} onClick={onClose}>
              Cancel
            </button>
          </div>
        </div>

        <div className={styles.body}>
          <RowList
            rows={rows}
            focusedIndex={focusedIndex}
            onFocusChange={setFocusedIndex}
            onReplace={replaceRow}
            onInsert={insertRow}
            onDelete={deleteRow}
            onUpdateCard={updateCardOp}
            onTriggerAutocomplete={triggerAutocomplete}
            onClose={onClose}
            acShow={acShow}
            acIndex={acIndex}
            acItems={acItems}
            acRowIndex={acRowIndex}
            onAcNavigate={(dir) =>
              setAcIndex((i) =>
                dir === "up"
                  ? Math.max(0, i - 1)
                  : Math.min(i + 1, acItems.length - 1),
              )
            }
            onAcSelect={executeAutocomplete}
            onAcClose={() => setAcShow(false)}
          />

          {acShow && (
            <AutocompleteMenu
              items={acItems}
              selectedIndex={acIndex}
              onSelect={executeAutocomplete}
              onClose={() => setAcShow(false)}
              rowIndex={acRowIndex}
            />
          )}
        </div>
      </div>
    </div>
  );
};

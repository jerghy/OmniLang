// poiesis/src/components/canvas/code/card-editor/RowList.tsx
import React, { useRef, useEffect } from "react";
import type { EditorRow } from "./row-utils";
import type { CodeOp } from "../../../../core/types/code-op";
import type { AutocompleteItem } from "./autocomplete-rules";
import { InputRow } from "./InputRow";
import { CardRow } from "./CardRow";
import styles from "./CodeCardEditor.module.css";

interface Props {
  rows: EditorRow[];
  focusedIndex: number;
  onFocusChange: (idx: number) => void;
  onReplace: (idx: number, row: EditorRow) => void;
  onInsert: (idx: number, row: EditorRow) => void;
  onDelete: (idx: number) => void;
  onUpdateCard: (idx: number, op: CodeOp) => void;
  onTriggerAutocomplete: (rowIdx: number, token: string) => void;
  onClose: () => void;
  // autocomplete
  acShow: boolean;
  acIndex: number;
  acItems: AutocompleteItem[];
  acRowIndex: number;
  onAcNavigate: (dir: "up" | "down") => void;
  onAcSelect: (item: AutocompleteItem) => void;
  onAcClose: () => void;
}

export const RowList: React.FC<Props> = ({
  rows,
  focusedIndex,
  onFocusChange,
  onReplace,
  onInsert,
  onDelete,
  onUpdateCard,
  onTriggerAutocomplete,
  onClose,
  acShow,
  acIndex,
  acItems,
  acRowIndex,
  onAcNavigate,
  onAcSelect,
  onAcClose,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);

  // Scroll focused row into view
  useEffect(() => {
    const el = containerRef.current?.children[focusedIndex] as HTMLElement | undefined;
    el?.scrollIntoView({ block: "nearest" });
  }, [focusedIndex]);

  const handleKeyDown = (e: React.KeyboardEvent, idx: number) => {
    if (acShow) {
      if (e.key === "ArrowDown") { e.preventDefault(); onAcNavigate("down"); return; }
      if (e.key === "ArrowUp") { e.preventDefault(); onAcNavigate("up"); return; }
      if (e.key === "Enter") { e.preventDefault(); onAcSelect(acItems[acIndex]); return; }
      if (e.key === "Escape") { e.preventDefault(); onAcClose(); return; }
      return;
    }

    if (e.key === "ArrowDown") {
      e.preventDefault();
      onFocusChange(Math.min(idx + 1, rows.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      onFocusChange(Math.max(idx - 1, 0));
    } else if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  };

  return (
    <div ref={containerRef} className={styles.rowList} tabIndex={0} onKeyDown={(e) => handleKeyDown(e, focusedIndex)}>
      {rows.map((row, idx) => {
        const isFocused = idx === focusedIndex;
        const indent = row.kind === "card" ? row.indent : row.kind === "close-brace" ? row.indent : 0;

        switch (row.kind) {
          case "input":
            return (
              <InputRow
                key={row.id}
                text={row.text}
                indent={0}
                isFocused={isFocused}
                onChange={(text) => onReplace(idx, { kind: "input", id: row.id, text })}
                onFocus={() => onFocusChange(idx)}
                onTriggerAutocomplete={(token) => onTriggerAutocomplete(idx, token)}
                onEnter={() => {
                  // if ac is showing, select; otherwise insert empty below
                  if (acShow) onAcSelect(acItems[acIndex]);
                  else onInsert(idx, { kind: "input", id: `in-${Date.now()}`, text: "" });
                }}
                onBackspaceEmpty={() => {
                  if (rows.length > 1) {
                    onDelete(idx);
                    onFocusChange(Math.max(idx - 1, 0));
                  }
                }}
              />
            );
          case "card":
            return (
              <CardRow
                key={row.op.pid}
                op={row.op}
                indent={indent}
                isFocused={isFocused}
                onUpdate={(op) => onUpdateCard(idx, op)}
                onFocus={() => onFocusChange(idx)}
                onInsertBelow={() => onInsert(idx, { kind: "input", id: `in-${Date.now()}`, text: "" })}
                onDelete={() => {
                  const wasBlock = row.op.kind === "if_block" || row.op.kind === "while" || row.op.kind === "for_in";
                  if (wasBlock) {
                    // remove header + body rows + close-brace
                    let endIdx = idx + 1;
                    let depth = indent + 1;
                    while (endIdx < rows.length) {
                      const r = rows[endIdx];
                      if (r.kind === "close-brace" && r.indent === indent) { endIdx++; break; }
                      endIdx++;
                    }
                    setRowsAndFocus(rows, onReplace, onFocusChange, idx, endIdx, indent);
                  } else {
                    // degrade to input row
                    onReplace(idx, { kind: "input", id: `in-${Date.now()}`, text: "" });
                  }
                }}
              />
            );
          case "close-brace":
            return (
              <div
                key={`close-${idx}`}
                className={styles.closeBrace}
                style={{ paddingLeft: indent * 20 + 8 }}
                onClick={() => onFocusChange(idx)}
              >
                {"}"}
              </div>
            );
          case "empty":
            return (
              <div
                key={row.id}
                className={styles.emptyRow}
                onClick={() => onFocusChange(idx)}
              />
            );
          default:
            return null;
        }
      })}
    </div>
  );
};

function setRowsAndFocus(
  rows: EditorRow[],
  onReplace: (idx: number, row: EditorRow) => void,
  onFocusChange: (idx: number) => void,
  start: number,
  end: number,
  indent: number,
) {
  // For block deletion, we degrade the header to an input row
  // This is called from CardRow — the parent CodeCardEditor handles row mutations
  // We use a workaround: replace start with input, the rest will be handled by the parent
  onReplace(start, { kind: "input", id: `in-${Date.now()}`, text: "" });
  onFocusChange(start);
}

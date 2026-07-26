// poiesis/src/components/canvas/code/BlockEditor.tsx
// Task 8: Rewritten — pure SVG blocks with BlockModel, no Blockly dependency.
// Wires together BlockModel + KeyboardHandler + ClipboardManager +
// SelectionOverlay + FuzzySearchPopup + raw input modal.

import React, { useCallback, useEffect, useRef, useState } from "react";
import type { CodeOp } from "../../../core/types/code-op";
import { BlockModel } from "./blocks/BlockModel";
import type { SelectionTarget } from "./blocks/BlockModel";
import { KeyboardHandler } from "./blocks/KeyboardHandler";
import type { KBCallbacks } from "./blocks/KeyboardHandler";
import { ClipboardManager } from "./blocks/ClipboardManager";
import { SelectionOverlay } from "./blocks/SelectionOverlay";
import {
  FuzzySearchPopup,
  getItemsForContext,
} from "./blocks/fields/FuzzySearchPopup";
import type { SearchItem } from "./blocks/fields/FuzzySearchPopup";
import { CodeEditorModal } from "./CodeEditorModal";
import {
  LetVarBlock,
  SetVarBlock,
  ReturnBlock,
  CallBlock,
  AwaitBlock,
  BreakBlock,
  ContinueBlock,
  CommentBlock,
  RawBlock,
  IfBlock,
  WhileBlock,
  ForInBlock,
} from "./blocks/BlockSvg/index";
import styles from "./BlockEditor.module.css";

// ── Props (unchanged external interface) ──

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

// ── Shared ClipboardManager (module-level, stable instance) ──

const clipboard = new ClipboardManager();

// ── BlockEditor component ──

export const BlockEditor: React.FC<Props> = ({
  elementId,
  initialOps,
  initialSource,
  language = "omnilang",
  onSave,
  onClose,
}) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const [showMonaco, setShowMonaco] = useState(false);
  const [showRawInput, setShowRawInput] = useState(false);
  const [rawInputText, setRawInputText] = useState("");
  const [opsCount, setOpsCount] = useState(initialOps.length);
  const [editingRawPid, setEditingRawPid] = useState<string | null>(null);

  const khRef = useRef<KeyboardHandler | null>(null);
  const blockRefs = useRef<Map<string, SVGGElement>>(new Map());

  // ── Initialize model (lazily, once via useState initializer) ──

  const [model] = useState(() => {
    const m = new BlockModel(initialOps);
    m.onChange = () => setOpsCount(m.getOps().length);
    return m;
  });

  // ── getElement callback: maps SelectionTarget -> DOM element ──

  const getElement = useCallback(
    (target: SelectionTarget): HTMLElement | SVGElement | null => {
      switch (target.type) {
        case "block": {
          const el = document.querySelector(
            `[data-block-id="${target.blockId}"]`,
          );
          return el as SVGElement | null;
        }
        case "slot": {
          const blockEl = document.querySelector(
            `[data-block-id="${target.blockId}"]`,
          );
          if (!blockEl) return null;
          const el = blockEl.querySelector(
            `[data-slot-id="${target.slotName}"]`,
          );
          return el as SVGElement | null;
        }
        case "gap": {
          const el = containerRef.current?.querySelector(
            `[data-gap-position="${target.position}"]`,
          );
          return el as HTMLElement | null;
        }
        default:
          return null;
      }
    },
    [],
  );

  // ── Keyboard handler ──

  useEffect(() => {
    const container = containerRef.current;
    if (!container) return;

    const callbacks: KBCallbacks = {
      onFuzzySearch: (query: string, context: "slot" | "gap") => {
        const items = getItemsForContext(context);
        const popup = new FuzzySearchPopup(
          items,
          (item: SearchItem) => {
            if (context === "gap") {
              const target = model.getCurrentTarget();
              if (target?.type === "gap") {
                model.insertBlock(item.blockType, target.position);
              }
            } else {
              const target = model.getCurrentTarget();
              if (target?.type === "slot") {
                model.setSlotValue(target.blockIndex, target.slotName, {
                  kind: "text",
                  value: item.label,
                });
              }
            }
          },
          () => {
            /* close — no-op */
          },
        );
        popup.open(query);
      },
      onRawInput: () => setShowRawInput(true),
      onOpsChanged: () => setOpsCount(model.getOps().length),
    };

    // Wire up clipboard Ctrl+C/Ctrl+X/Ctrl+V in the container (capture phase
    // so these fire before KeyboardHandler and consume the event).
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.ctrlKey || e.metaKey) {
        if (e.key === "c" || e.key === "C") {
          e.preventDefault();
          const target = model.getCurrentTarget();
          if (target?.type === "block") {
            clipboard.copy(model.getOps()[target.blockIndex]);
          }
        } else if (e.key === "x" || e.key === "X") {
          e.preventDefault();
          const target = model.getCurrentTarget();
          if (target?.type === "block") {
            clipboard.copy(model.getOps()[target.blockIndex]);
            model.deleteBlock(target.blockIndex);
          }
        } else if (e.key === "v" || e.key === "V") {
          e.preventDefault();
          clipboard.paste().then((op) => {
            if (op) {
              const target = model.getCurrentTarget();
              if (target?.type === "gap") {
                model.insertCodeOp(op, target.position);
              }
            }
          });
        }
      }
    };
    container.addEventListener("keydown", onKeyDown, { capture: true });

    const kh = new KeyboardHandler(container, model, getElement, callbacks);
    khRef.current = kh;

    return () => {
      kh.dispose();
      container.removeEventListener("keydown", onKeyDown, { capture: true });
    };
  }, [elementId, model, getElement]);

  // ── Save ──

  const handleSave = useCallback(() => {
    onSave(model.getOps());
  }, [onSave, model]);

  // ── Render a single block from CodeOp ──

  const renderBlock = useCallback(
    (op: CodeOp): React.ReactNode => {
      const setRef = (el: SVGGElement | null) => {
        if (el) blockRefs.current.set(op.pid, el);
      };

      switch (op.kind) {
        case "declare_var":
          return <LetVarBlock key={op.pid} op={op} ref={setRef} />;
        case "set_var":
          return <SetVarBlock key={op.pid} op={op} ref={setRef} />;
        case "return":
          return <ReturnBlock key={op.pid} op={op} ref={setRef} />;
        case "call_fn":
          return <CallBlock key={op.pid} op={op} ref={setRef} />;
        case "await":
          return <AwaitBlock key={op.pid} op={op} ref={setRef} />;
        case "break":
          return <BreakBlock key={op.pid} op={op} ref={setRef} />;
        case "continue":
          return <ContinueBlock key={op.pid} op={op} ref={setRef} />;
        case "comment":
          return <CommentBlock key={op.pid} op={op} ref={setRef} />;
        case "raw":
          return (
            <RawBlock
              key={op.pid}
              op={op}
              ref={setRef}
              isEditing={editingRawPid === op.pid}
              onToggleEdit={() =>
                setEditingRawPid(
                  editingRawPid === op.pid ? null : op.pid,
                )
              }
              onEdit={(src: string) => {
                op.source = src;
                setEditingRawPid(null);
              }}
            />
          );
        case "if_block":
          return (
            <IfBlock
              key={op.pid}
              op={op}
              ref={setRef}
              renderBody={(body: CodeOp[]) => body.map(renderBlock)}
            />
          );
        case "while":
          return (
            <WhileBlock
              key={op.pid}
              op={op}
              ref={setRef}
              renderBody={(body: CodeOp[]) => body.map(renderBlock)}
            />
          );
        case "for_in":
          return (
            <ForInBlock
              key={op.pid}
              op={op}
              ref={setRef}
              renderBody={(body: CodeOp[]) => body.map(renderBlock)}
            />
          );
        default:
          return null;
      }
    },
    [editingRawPid],
  );

  // ── Raw input commit ──

  const commitRawInput = useCallback(() => {
    if (!rawInputText.trim()) {
      setShowRawInput(false);
      return;
    }
    model.insertBlock("raw_stmt", model.getOps().length);
    const ops = model.getOps();
    const lastOp = ops[ops.length - 1];
    if (lastOp.kind === "raw") lastOp.source = rawInputText;
    setShowRawInput(false);
    setRawInputText("");
  }, [rawInputText, model]);

  // ── Monaco fallback ──

  if (showMonaco) {
    return (
      <CodeEditorModal
        elementId={elementId}
        initialSource={initialSource}
        initialDescription=""
        language={language}
        onSave={async () => {
          onSave(initialOps);
          setShowMonaco(false);
        }}
        onClose={() => setShowMonaco(false)}
      />
    );
  }

  // ── Block editor mode ──

  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        <div className={styles.header}>
          <h3>Block Editor — {opsCount} ops</h3>
          <div>
            <button type="button" onClick={() => setShowMonaco(true)}>文本模式</button>
            <button type="button" onClick={handleSave}>Save</button>
            <button type="button" onClick={onClose}>Cancel</button>
          </div>
        </div>

        <div
          ref={containerRef}
          className={styles.container}
          style={{ overflowY: "auto", height: "100%" }}
        >
          <svg width="100%" height="100%">
            {/* Gap rows — before first block, between blocks, and after last block */}
        <g transform={`translate(8, 8)`}>
          {/* Gap before first block */}
          <rect
            data-gap-position={0}
            x={0} y={0}
            width="100%"
            height={12}
            fill="transparent"
          />
          {model.getOps().map((op, i) => (
            <g key={op.pid}>
              <g transform={`translate(0, ${i * 48 + 12})`}>
                {renderBlock(op)}
              </g>
              {/* Gap between blocks and after last block */}
              <g transform={`translate(0, ${i * 48 + 12 + 36})`}>
                <rect
                  data-gap-position={i + 0.5}
                  width="100%"
                  height={12}
                  fill="transparent"
                />
              </g>
            </g>
          ))}
          {/* Gap after last block */}
          <rect
            data-gap-position={model.getOps().length}
            x={0} y={model.getOps().length * 48 + 12}
            width="100%"
            height={12}
            fill="transparent"
          />
        </g>
          </svg>
        </div>

        <SelectionOverlay
          target={model.getCurrentTarget()}
          getElement={getElement}
        />

        {showRawInput && (
          <div className={styles.rawOverlay}>
            <textarea
              autoFocus
              value={rawInputText}
              onChange={(e) => setRawInputText(e.target.value)}
              placeholder="Enter raw OmniLang code..."
              aria-label="Raw code input"
            />
            <button type="button" onClick={() => setShowRawInput(false)}>Esc 取消</button>
            <button type="button" onClick={commitRawInput}>Enter 提交</button>
          </div>
        )}
      </div>
    </div>
  );
};

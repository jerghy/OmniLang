// poiesis/src/components/canvas/code/BlockEditor.tsx
// Task 9: BlockEditor — the React component that replaces CodeCardEditor.
// Wires together Blockly workspace + BlockAdapter + SelectionManager +
// GapManager + KeyboardHandler + FuzzySearchPopup + raw input modal.

import { useCallback, useEffect, useRef, useState } from "react";
import * as Blockly from "blockly";
import type { CodeOp } from "../../../core/types/code-op";
import { BlockAdapter } from "./blocks/BlockAdapter";
import { SelectionManager } from "./blocks/SelectionManager";
import { GapManager } from "./blocks/GapManager";
import { KeyboardHandler } from "./blocks/KeyboardHandler";
import type { KeyboardHandlerCallbacks } from "./blocks/KeyboardHandler";
import {
  FuzzySearchPopup,
  getItemsForContext,
} from "./blocks/fields/FuzzySearchPopup";
import type { SearchItem } from "./blocks/fields/FuzzySearchPopup";
import { CodeEditorModal } from "./CodeEditorModal";
import styles from "./BlockEditor.module.css";

// ── Props ──

interface Props {
  elementId: string;
  initialOps: CodeOp[];
  initialSource: string;
  language?: "typescript" | "omnilang";
  onSave: (operations: CodeOp[]) => void;
  onClose: () => void;
}

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
  const [opsCount, setOpsCount] = useState(initialOps.length);

  // Refs for manager instances (stable across renders)
  const wsRef = useRef<Blockly.WorkspaceSvg | null>(null);
  const adapterRef = useRef<BlockAdapter>(new BlockAdapter());
  const smRef = useRef<SelectionManager | null>(null);
  const gmRef = useRef<GapManager | null>(null);
  const khRef = useRef<KeyboardHandler | null>(null);
  const opsRef = useRef<CodeOp[]>(initialOps);

  // ── Sync CodeOp[] from workspace ──

  const syncOps = useCallback(() => {
    const ws = wsRef.current;
    if (!ws) return;
    const ops = adapterRef.current.fromBlocks(ws);
    opsRef.current = ops;
    setOpsCount(ops.length);
  }, []);

  // ── Initialize Blockly workspace ──

  useEffect(() => {
    const container = containerRef.current;
    if (!container || wsRef.current) return;

    // 1. Create workspace with Zelos renderer, no toolbox (keyboard-only)
    const ws = Blockly.inject(container, {
      renderer: "zelos",
      scrollbars: true,
      grid: {
        spacing: 20,
        length: 3,
        colour: "#e8e7e2",
        snap: false,
      },
      zoom: { controls: true, wheel: true, startScale: 1.0 },
      move: { scrollbars: true, drag: true, wheel: true },
    });

    // 2. Populate with initial ops
    if (initialOps.length > 0) {
      adapterRef.current.toBlocks(initialOps, ws);
    }

    // 3. Create managers
    const sm = new SelectionManager(ws);
    const gm = new GapManager(ws);
    gm.initializeForBlockCount(ws.getTopBlocks(true).length);

    // 4. Create KeyboardHandler with callbacks
    const callbacks: KeyboardHandlerCallbacks = {
      onFuzzySearch: (query: string, context: "slot" | "gap") => {
        const items = getItemsForContext(context);
        const popup = new FuzzySearchPopup(
          items,
          (item: SearchItem) => {
            // Create block at current position
            const block = ws.newBlock(item.blockType);
            block.initSvg();
            block.render();
            block.moveBy(20, 20);

            // If the current target is a gap, insert the block there
            const target = sm.getCurrentTarget();
            if (target?.type === "gap") {
              // Place block at the gap position
              const topBlocks = ws.getTopBlocks(true);
              const gapPos = target.position;
              const afterIndex = Math.floor(gapPos);

              if (afterIndex < topBlocks.length - 1) {
                // Connect to the chain at the gap position
                const beforeBlock = topBlocks[afterIndex];
                const afterBlock = topBlocks[afterIndex + 1];

                if (beforeBlock.nextConnection && afterBlock.previousConnection) {
                  // Disconnect the existing chain, insert new block
                  beforeBlock.nextConnection.disconnect();
                  beforeBlock.nextConnection.connect(
                    block.previousConnection!,
                  );
                  block.nextConnection?.connect(afterBlock.previousConnection);
                }
              } else if (topBlocks.length > 0) {
                // Append at the end
                const lastBlock = topBlocks[topBlocks.length - 1];
                if (lastBlock.nextConnection && block.previousConnection) {
                  lastBlock.nextConnection.connect(block.previousConnection);
                }
              }
            }

            syncOps();
          },
          () => {
            /* close — no-op */
          },
        );
        popup.open(query);
      },
      onRawInput: () => setShowRawInput(true),
      onOpsChanged: () => syncOps(),
    };

    const kh = new KeyboardHandler(ws, sm, gm, callbacks);

    // Store refs
    smRef.current = sm;
    gmRef.current = gm;
    khRef.current = kh;
    wsRef.current = ws;

    // Initial ops sync
    syncOps();

    // Cleanup on unmount
    return () => {
      kh.dispose();
      sm.dispose();
      ws.dispose();
      wsRef.current = null;
      smRef.current = null;
      gmRef.current = null;
      khRef.current = null;
    };

    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [elementId]);

  // ── Handle save ──

  const handleSave = useCallback(() => {
    onSave(opsRef.current);
  }, [onSave]);

  // ── Monaco fallback save handler ──

  const handleMonacoSave = useCallback(
    async (_source: string, _desc: string) => {
      // In Monaco mode, we save the source text
      onSave(initialOps);
      setShowMonaco(false);
    },
    [onSave, initialOps],
  );

  // ── Raw input ──

  const [rawInputText, setRawInputText] = useState("");

  const commitRawInput = useCallback(() => {
    if (!rawInputText.trim()) {
      setShowRawInput(false);
      return;
    }

    // Add raw operation to workspace blocks
    const ws = wsRef.current;
    if (ws) {
      const block = ws.newBlock("raw_stmt");
      block.setFieldValue(rawInputText, "SOURCE");
      block.initSvg();
      block.render();
      block.moveBy(20, 20);

      // Append to the end of top blocks
      const topBlocks = ws.getTopBlocks(true);
      if (topBlocks.length > 1) {
        const lastBlock = topBlocks[topBlocks.length - 2]; // before the one we just added
        if (lastBlock.nextConnection && block.previousConnection) {
          lastBlock.nextConnection.connect(block.previousConnection);
        }
      }

      syncOps();
    }

    setShowRawInput(false);
    setRawInputText("");
  }, [rawInputText, syncOps]);

  const cancelRawInput = useCallback(() => {
    setShowRawInput(false);
    setRawInputText("");
  }, []);

  // ── Monaco fallback mode ──

  if (showMonaco) {
    return (
      <CodeEditorModal
        elementId={elementId}
        initialSource={initialSource}
        initialDescription=""
        language={language}
        onSave={handleMonacoSave}
        onClose={() => setShowMonaco(false)}
      />
    );
  }

  // ── Block editor mode ──

  return (
    <div className={styles.overlay}>
      <div className={styles.editor}>
        {/* Header */}
        <div className={styles.header}>
          <h3 className={styles.title}>
            Block Editor — {opsCount} ops
          </h3>
          <div className={styles.actions}>
            <button
              type="button"
              className={styles.textModeBtn}
              onClick={() => setShowMonaco(true)}
            >
              文本模式
            </button>
            <button
              type="button"
              className={styles.saveBtn}
              onClick={handleSave}
            >
              Save
            </button>
            <button
              type="button"
              className={styles.cancelBtn}
              onClick={onClose}
            >
              Cancel
            </button>
          </div>
        </div>

        {/* Blockly workspace container */}
        <div ref={containerRef} className={styles.blocklyContainer} />

        {/* Raw input modal */}
        {showRawInput && (
          <div className={styles.rawInputOverlay}>
            <div className={styles.rawInputModal}>
              <textarea
                className={styles.rawInputTextarea}
                value={rawInputText}
                onChange={(e) => setRawInputText(e.target.value)}
                placeholder="Enter raw OmniLang code..."
                autoFocus
              />
              <div className={styles.rawInputActions}>
                <button
                  type="button"
                  className={styles.rawInputCancelBtn}
                  onClick={cancelRawInput}
                >
                  Esc 取消
                </button>
                <button
                  type="button"
                  className={styles.rawInputSubmitBtn}
                  onClick={commitRawInput}
                >
                  Enter 提交
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

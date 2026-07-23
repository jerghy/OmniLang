// poiesis/src/components/panels/inspector/HandlerModal.tsx
// Floating modal for editing an event handler. Two tabs:
// 1. 绑定函数 — select a function/action reference
// 2. 写表达式 — write an inline closure via Monaco editor

import { useState, useCallback, useEffect } from "react";
import { ExpressionEditor } from "./ExpressionEditor";
import type { HandlerItem } from "./useHandlerItems";
import { FunctionBindingTab } from "./FunctionBindingTab";
import styles from "./HandlerModal.module.css";

interface HandlerModalProps {
  eventName: string;
  initialHandler: string;
  handlerItems: HandlerItem[];
  onConfirm: (handlerString: string) => void;
  onClose: () => void;
}

type TabMode = "function" | "expression";

/** Detect if a handler string is likely an inline expression (starts with `(` or contains `=>`). */
function looksLikeExpression(handler: string): boolean {
  const trimmed = handler.trim();
  return trimmed.startsWith("(") || trimmed.includes("=>") || trimmed.includes("{");
}

export function HandlerModal({
  eventName,
  initialHandler,
  handlerItems,
  onConfirm,
  onClose,
}: HandlerModalProps) {
  // Determine initial tab
  const initialTab: TabMode =
    !initialHandler || looksLikeExpression(initialHandler) ? "expression" : "function";

  const [tab, setTab] = useState<TabMode>(initialTab);

  // Function picker state
  const [selectedFn, setSelectedFn] = useState<HandlerItem | null>(null);
  const [fnArgs, setFnArgs] = useState<Record<string, string>>({});
  const [pickerOpen, setPickerOpen] = useState(false);

  // Expression editor state
  const [expression, setExpression] = useState(
    initialTab === "expression" && initialHandler
      ? initialHandler
      : "() => {\n  \n}"
  );

  // Parse initialHandler to pre-fill function picker if in function mode
  useEffect(() => {
    if (initialTab === "function" && initialHandler) {
      // Try to match: handlerName or handlerName(args)
      const match = initialHandler.match(/^(\w[\w.]*)\(?(.*?)\)?$/);
      if (match) {
        const fnName = match[1];
        const found = handlerItems.find((item) => {
          // Match by name or by source.name pattern
          return item.name === fnName || item.displayName.startsWith(fnName);
        });
        if (found) {
          setSelectedFn(found);
          // Parse args if present
          if (match[2]) {
            const argParts = match[2].split(",").map((s) => s.trim()).filter(Boolean);
            const args: Record<string, string> = {};
            found.params.forEach((p, i) => {
              if (i < argParts.length) {
                args[p.name] = argParts[i].replace(/^"(.*)"$/, "$1");
              }
            });
            // eslint-disable-next-line react-doctor/no-adjust-state-on-prop-change
            setFnArgs(args);
          }
        }
      }
    }
  }, [initialTab, initialHandler, handlerItems]);

  // Build the final handler string
  const buildHandlerString = useCallback((): string => {
    if (tab === "expression") {
      return expression.trim();
    }
    // Function mode
    if (!selectedFn) return "";
    if (selectedFn.params.length === 0) {
      return selectedFn.source === "store"
        ? `${selectedFn.sourceName}.${selectedFn.name}`
        : selectedFn.name;
    }
    // With args
    const argValues = selectedFn.params.map((p) => {
      const val = fnArgs[p.name] ?? "";
      return val;
    });
    const prefix =
      selectedFn.source === "store"
        ? `${selectedFn.sourceName}.${selectedFn.name}`
        : selectedFn.name;
    return `${prefix}(${argValues.join(", ")})`;
  }, [tab, expression, selectedFn, fnArgs]);

  // Escape key to close
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const handleConfirm = useCallback(() => {
    const str = buildHandlerString();
    if (str) onConfirm(str);
  }, [buildHandlerString, onConfirm]);

  const handleFnSelect = useCallback((item: HandlerItem) => {
    setSelectedFn(item);
    setPickerOpen(false);
    // Reset args when selecting a new function
    const args: Record<string, string> = {};
    item.params.forEach((p) => {
      args[p.name] = "";
    });
    setFnArgs(args);
  }, []);

  const handleArgChange = useCallback((paramName: string, value: string) => {
    setFnArgs((prev) => ({ ...prev, [paramName]: value }));
  }, []);

  return (
    <>
      {/* Backdrop */}
      <button
        type="button"
        tabIndex={0}
        aria-label="Close handler modal"
        onClick={onClose}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); } }}
        className={styles.backdrop}
      />
      {/* Modal */}
      <div
        onClick={(e) => e.stopPropagation()}
        className={styles.modal}
      >
        {/* Title */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "14px 20px",
            borderBottom: "1px solid var(--ol-border)",
          }}
        >
          <span style={{ fontSize: 14, fontWeight: 600, color: "var(--ol-gray-800)" }}>
            编辑 {eventName} 处理器
          </span>
          <button type="button"
            onClick={onClose}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              fontSize: 18,
              color: "var(--ol-text-secondary)",
              padding: "4px 8px",
            }}
          >
            ✕
          </button>
        </div>

        {/* Tab switcher */}
        <div
          style={{
            display: "flex",
            gap: 0,
            borderBottom: "1px solid var(--ol-border)",
            padding: "0 20px",
          }}
        >
          <TabButton
            active={tab === "function"}
            onClick={() => setTab("function")}
            label="绑定函数"
          />
          <TabButton
            active={tab === "expression"}
            onClick={() => setTab("expression")}
            label="写表达式"
          />
        </div>

        {/* Tab content */}
        <div style={{ padding: 16, overflowY: "auto", flex: 1 }}>
          {tab === "function" && (
            <FunctionBindingTab
              selectedFn={selectedFn}
              fnArgs={fnArgs}
              pickerOpen={pickerOpen}
              handlerItems={handlerItems}
              onTogglePicker={() => setPickerOpen(!pickerOpen)}
              onSelectFn={handleFnSelect}
              onPickerClose={() => setPickerOpen(false)}
              onArgChange={handleArgChange}
            />
          )}

          {tab === "expression" && (
            <ExpressionEditor
              initialValue={expression}
              onChange={setExpression}
              height={260}
            />
          )}
        </div>

        {/* Action buttons */}
        <div
          style={{
            display: "flex",
            justifyContent: "flex-end",
            gap: 8,
            padding: "12px 20px",
            borderTop: "1px solid var(--ol-border)",
          }}
        >
          <button type="button"
            onClick={onClose}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              border: "1px solid var(--ol-border-hover)",
              borderRadius: 6,
              background: "var(--ol-surface)",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            取消
          </button>
          <button type="button"
            onClick={handleConfirm}
            className={styles.confirmBtn}
          >
            确认
          </button>
        </div>
      </div>
    </>
  );
}

function TabButton({
  active,
  onClick,
  label,
}: {
  active: boolean;
  onClick: () => void;
  label: string;
}) {
  return (
    <button type="button"
      onClick={onClick}
      style={{
        padding: "10px 16px",
        fontSize: 13,
        fontWeight: active ? 600 : 400,
        border: "none",
        borderBottom: active ? "2px solid var(--ol-color-primary)" : "2px solid transparent",
        background: "none",
        cursor: "pointer",
        color: active ? "var(--ol-color-primary)" : "var(--ol-text-secondary)",
        fontFamily: "inherit",
      }}
    >
      {active ? "● " : "○ "}
      {label}
    </button>
  );
}

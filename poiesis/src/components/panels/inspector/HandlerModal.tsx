// poiesis/src/components/panels/inspector/HandlerModal.tsx
// Floating modal for editing an event handler. Two tabs:
// 1. 绑定函数 — select a function/action reference
// 2. 写表达式 — write an inline closure via Monaco editor

import { useState, useCallback, useEffect } from "react";
import { FunctionPicker } from "./FunctionPicker";
import { ExpressionEditor } from "./ExpressionEditor";
import type { HandlerItem } from "./useHandlerItems";

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
      <div
        onClick={onClose}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 9998,
          background: "rgba(0,0,0,0.4)",
        }}
      />
      {/* Modal */}
      <div
        onClick={(e) => e.stopPropagation()}
        style={{
          position: "fixed",
          top: "50%",
          left: "50%",
          transform: "translate(-50%, -50%)",
          zIndex: 9999,
          background: "#fff",
          borderRadius: 12,
          boxShadow: "0 8px 32px rgba(0,0,0,0.2)",
          width: 560,
          maxHeight: "80vh",
          display: "flex",
          flexDirection: "column",
          overflow: "hidden",
        }}
      >
        {/* Title */}
        <div
          style={{
            display: "flex",
            justifyContent: "space-between",
            alignItems: "center",
            padding: "14px 20px",
            borderBottom: "1px solid #e2e8f0",
          }}
        >
          <span style={{ fontSize: 14, fontWeight: 600, color: "#1e293b" }}>
            编辑 {eventName} 处理器
          </span>
          <button
            onClick={onClose}
            style={{
              border: "none",
              background: "none",
              cursor: "pointer",
              fontSize: 18,
              color: "#94a3b8",
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
            borderBottom: "1px solid #e2e8f0",
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
            <div style={{ display: "flex", flexDirection: "column", gap: 12 }}>
              {/* Function/Action selector */}
              <div>
                <label
                  style={{
                    display: "block",
                    fontSize: 11,
                    fontWeight: 600,
                    color: "#64748b",
                    marginBottom: 4,
                  }}
                >
                  函数/Action
                </label>
                <div style={{ position: "relative" }}>
                  <button
                    onClick={() => setPickerOpen(!pickerOpen)}
                    style={{
                      width: "100%",
                      display: "flex",
                      justifyContent: "space-between",
                      alignItems: "center",
                      padding: "8px 12px",
                      fontSize: 13,
                      fontFamily: "var(--dp-font-mono, monospace)",
                      border: "1px solid #d1d5db",
                      borderRadius: 6,
                      background: "#fff",
                      cursor: "pointer",
                      textAlign: "left",
                      color: selectedFn ? "#1e293b" : "#94a3b8",
                    }}
                  >
                    <span>
                      {selectedFn ? selectedFn.displayName : "选择函数或 Action..."}
                    </span>
                    <span style={{ color: "#94a3b8" }}>▼</span>
                  </button>
                  {pickerOpen && (
                    <FunctionPicker
                      items={handlerItems}
                      selectedName={selectedFn?.name ?? null}
                      onSelect={handleFnSelect}
                      onClose={() => setPickerOpen(false)}
                    />
                  )}
                </div>
              </div>

              {/* Parameter inputs (shown when selected function has params) */}
              {selectedFn && selectedFn.params.length > 0 && (
                <div>
                  <label
                    style={{
                      display: "block",
                      fontSize: 11,
                      fontWeight: 600,
                      color: "#64748b",
                      marginBottom: 4,
                    }}
                  >
                    参数
                  </label>
                  <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
                    {selectedFn.params.map((p) => (
                      <div
                        key={p.name}
                        style={{ display: "flex", alignItems: "center", gap: 8 }}
                      >
                        <label
                          style={{
                            fontSize: 11,
                            color: "#64748b",
                            minWidth: 40,
                            fontFamily: "var(--dp-font-mono, monospace)",
                          }}
                        >
                          {p.name}
                        </label>
                        <input
                          type="text"
                          value={fnArgs[p.name] ?? ""}
                          onChange={(e) => handleArgChange(p.name, e.target.value)}
                          placeholder={p.type}
                          style={{
                            flex: 1,
                            padding: "6px 10px",
                            fontSize: 12,
                            border: "1px solid #d1d5db",
                            borderRadius: 4,
                            outline: "none",
                            fontFamily: "var(--dp-font-mono, monospace)",
                          }}
                          onFocus={(e) => (e.target.style.borderColor = "#6366F1")}
                          onBlur={(e) => (e.target.style.borderColor = "#d1d5db")}
                        />
                      </div>
                    ))}
                  </div>
                </div>
              )}
            </div>
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
            borderTop: "1px solid #e2e8f0",
          }}
        >
          <button
            onClick={onClose}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              border: "1px solid #d1d5db",
              borderRadius: 6,
              background: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            取消
          </button>
          <button
            onClick={handleConfirm}
            style={{
              padding: "8px 20px",
              fontSize: 13,
              fontWeight: 600,
              border: "none",
              borderRadius: 6,
              background: "#6366F1",
              color: "#fff",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
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
    <button
      onClick={onClick}
      style={{
        padding: "10px 16px",
        fontSize: 13,
        fontWeight: active ? 600 : 400,
        border: "none",
        borderBottom: active ? "2px solid #6366F1" : "2px solid transparent",
        background: "none",
        cursor: "pointer",
        color: active ? "#6366F1" : "#64748b",
        fontFamily: "inherit",
      }}
    >
      {active ? "● " : "○ "}
      {label}
    </button>
  );
}

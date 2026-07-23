// components/panels/inspector/FunctionBindingTab.tsx
// The "bind function" tab content for HandlerModal.

import styles from "./FunctionBindingTab.module.css";

import { FunctionPicker } from "./FunctionPicker";
import type { HandlerItem } from "./useHandlerItems";

interface FunctionBindingTabProps {
  selectedFn: HandlerItem | null;
  fnArgs: Record<string, string>;
  pickerOpen: boolean;
  handlerItems: HandlerItem[];
  onTogglePicker: () => void;
  onSelectFn: (item: HandlerItem) => void;
  onPickerClose: () => void;
  onArgChange: (paramName: string, value: string) => void;
}

export function FunctionBindingTab({
  selectedFn,
  fnArgs,
  pickerOpen,
  handlerItems,
  onTogglePicker,
  onSelectFn,
  onPickerClose,
  onArgChange,
}: FunctionBindingTabProps) {
  return (
    <div className={styles.section}>
      {/* Function/Action selector */}
      <div>
        <span className={styles.fieldLabel}>
          函数/Action
        </span>
        <div style={{ position: "relative" }}>
          <button type="button"
            onClick={onTogglePicker}
            className={styles.pickerBtn}
            style={{ color: selectedFn ? "#1e293b" : "#94a3b8" }}
          >
            <span>
              {selectedFn ? selectedFn.displayName : "选择函数或 Action..."}
            </span>
            <span className={styles.pickerBtnArrow}>▼</span>
          </button>
          {pickerOpen && (
            <FunctionPicker
              items={handlerItems}
              selectedName={selectedFn?.name ?? null}
              onSelect={onSelectFn}
              onClose={onPickerClose}
            />
          )}
        </div>
      </div>

      {/* Parameter inputs (shown when selected function has params) */}
      {selectedFn && selectedFn.params.length > 0 && (
        <div>
          <span className={styles.paramsLabel}>
            参数
          </span>
          <div style={{ display: "flex", flexDirection: "column", gap: 6 }}>
            {selectedFn.params.map((p) => (
              <div key={p.name} className={styles.paramRow}>
                <label className={styles.paramLabelInner}>
                  <span className={styles.paramName}>
                    {p.name}
                  </span>
                  <input
                    type="text"
                    value={fnArgs[p.name] ?? ""}
                    onChange={(e) => onArgChange(p.name, e.target.value)}
                    placeholder={p.type}
                    className={styles.paramInput}
                    onFocus={(e) => (e.target.style.borderColor = "#6366F1")}
                    onBlur={(e) => (e.target.style.borderColor = "#d1d5db")}
                  />
                </label>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}

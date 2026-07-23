// components/design/NewFileModal.tsx
// Dialog for creating new .omni files (store or code type).

import { useState, useCallback } from "react";
import styles from "./NewFileModal.module.css";

interface NewFileModalProps {
  onCreate: (name: string, type: "store" | "code") => void;
  onClose: () => void;
}

export function NewFileModal({ onCreate, onClose }: NewFileModalProps) {
  const [newFileType, setNewFileType] = useState<"store" | "code">("code");
  const [newFileName, setNewFileName] = useState("");

  const handleCreate = useCallback(() => {
    if (!newFileName.trim()) return;
    onCreate(newFileName.trim(), newFileType);
    setNewFileName("");
  }, [newFileName, newFileType, onCreate]);

  return (
    <>
      <button
        type="button"
        tabIndex={0}
        aria-label="Close new file modal"
        onClick={() => { onClose(); setNewFileName(""); }}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); setNewFileName(""); } }}
        className={styles.backdrop}
      />
      <div className={styles.modal}>
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "var(--ol-gray-800)" }}>
          新建文件
        </div>

        {/* File type selector */}
        <div style={{ marginBottom: 12 }}>
          <div style={{ fontSize: 12, color: "var(--ol-text-secondary)", marginBottom: 6 }}>文件类型</div>
          <div style={{ display: "flex", gap: 8 }}>
            <button type="button"
              onClick={() => setNewFileType("store")}
              style={{
                flex: 1,
                padding: "6px 10px",
                border: newFileType === "store" ? "2px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
                borderRadius: 6,
                background: newFileType === "store" ? "#EEF2FF" : "var(--ol-surface)",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: newFileType === "store" ? 600 : 400,
                color: newFileType === "store" ? "#4338CA" : "var(--ol-text-secondary)",
                fontFamily: "inherit",
              }}
            >
              🗄 Store
            </button>
            <button type="button"
              onClick={() => setNewFileType("code")}
              style={{
                flex: 1,
                padding: "6px 10px",
                border: newFileType === "code" ? "2px solid var(--ol-color-primary)" : "1px solid var(--ol-border)",
                borderRadius: 6,
                background: newFileType === "code" ? "#EEF2FF" : "var(--ol-surface)",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: newFileType === "code" ? 600 : 400,
                color: newFileType === "code" ? "#4338CA" : "var(--ol-text-secondary)",
                fontFamily: "inherit",
              }}
            >
              {"</>"} Code
            </button>
          </div>
        </div>

        {/* File name input */}
        <div style={{ marginBottom: 4 }}>
          <div style={{ fontSize: 12, color: "var(--ol-text-secondary)", marginBottom: 6 }}>文件名</div>
          <div style={{ display: "flex", alignItems: "center", gap: 2 }}>
            <input
              type="text"
              value={newFileName}
              onChange={(e) => setNewFileName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
                if (e.key === "Escape") {
                  onClose();
                  setNewFileName("");
                }
              }}
              placeholder="my-file"
              aria-label="File name"
              style={{
                flex: 1,
                padding: "5px 8px",
                border: "1px solid var(--ol-border-hover)",
                borderRadius: 4,
                fontSize: 12,
                fontFamily: "inherit",
                outline: "2px solid transparent",
              }}
            />
            <span style={{ fontSize: 12, color: "var(--ol-text-muted)" }}>.omni</span>
          </div>
        </div>

        {/* Target path preview */}
        <div
          style={{
            fontSize: 12,
            color: "var(--ol-text-muted)",
            marginBottom: 12,
            padding: "2px 0",
          }}
        >
          存储路径: {newFileType === "store" ? "src/stores" : "src/lib"}/
          {newFileName.trim() ? `${newFileName.trim().replace(/\.omni$/i, "")}.omni` : "..."}
        </div>

        {/* Action buttons */}
        <div style={{ display: "flex", gap: 8, justifyContent: "flex-end" }}>
          <button type="button"
            onClick={() => { onClose(); setNewFileName(""); }}
            style={{
              padding: "5px 14px",
              fontSize: 12,
              border: "1px solid var(--ol-border-hover)",
              borderRadius: 6,
              background: "var(--ol-surface)",
              cursor: "pointer",
              fontFamily: "inherit",
            }}
          >
            返回
          </button>
          <button type="button"
            onClick={handleCreate}
            disabled={!newFileName.trim()}
            style={{
              padding: "5px 14px",
              fontSize: 12,
              border: "none",
              borderRadius: 6,
              background: newFileName.trim() ? "var(--ol-color-primary)" : "var(--ol-border-hover)",
              color: "var(--ol-surface)",
              cursor: newFileName.trim() ? "pointer" : "default",
              fontWeight: 600,
              fontFamily: "inherit",
            }}
          >
            创建
          </button>
        </div>
      </div>
    </>
  );
}

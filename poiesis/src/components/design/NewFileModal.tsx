// components/design/NewFileModal.tsx
// Dialog for creating new .omni files (store or code type).

import { useState, useCallback } from "react";

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
      <div
        role="button"
        tabIndex={0}
        aria-label="Close new file modal"
        onClick={() => { onClose(); setNewFileName(""); }}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onClose(); setNewFileName(""); } }}
        style={{
          position: "fixed",
          inset: 0,
          zIndex: 9998,
          background: "transparent",
        }}
      />
      <div
        style={{
          position: "fixed",
          top: 44,
          left: 8,
          zIndex: 9999,
          background: "#fff",
          borderRadius: 8,
          boxShadow: "0 4px 20px rgba(0,0,0,0.15)",
          minWidth: 280,
          padding: "16px",
        }}
      >
        <div style={{ fontSize: 13, fontWeight: 600, marginBottom: 12, color: "#1e293b" }}>
          新建文件
        </div>

        {/* File type selector */}
        <div style={{ marginBottom: 12 }}>
          <div style={{ fontSize: 12, color: "#6b7280", marginBottom: 6 }}>文件类型</div>
          <div style={{ display: "flex", gap: 8 }}>
            <button type="button"
              onClick={() => setNewFileType("store")}
              style={{
                flex: 1,
                padding: "6px 10px",
                border: newFileType === "store" ? "2px solid #6366F1" : "1px solid #e2e8f0",
                borderRadius: 6,
                background: newFileType === "store" ? "#EEF2FF" : "#fff",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: newFileType === "store" ? 600 : 400,
                color: newFileType === "store" ? "#4338CA" : "#64748b",
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
                border: newFileType === "code" ? "2px solid #6366F1" : "1px solid #e2e8f0",
                borderRadius: 6,
                background: newFileType === "code" ? "#EEF2FF" : "#fff",
                cursor: "pointer",
                fontSize: 12,
                fontWeight: newFileType === "code" ? 600 : 400,
                color: newFileType === "code" ? "#4338CA" : "#64748b",
                fontFamily: "inherit",
              }}
            >
              {"</>"} Code
            </button>
          </div>
        </div>

        {/* File name input */}
        <div style={{ marginBottom: 4 }}>
          <div style={{ fontSize: 12, color: "#6b7280", marginBottom: 6 }}>文件名</div>
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
                border: "1px solid #d1d5db",
                borderRadius: 4,
                fontSize: 12,
                fontFamily: "inherit",
                outline: "2px solid transparent",
              }}
            />
            <span style={{ fontSize: 12, color: "#94a3b8" }}>.omni</span>
          </div>
        </div>

        {/* Target path preview */}
        <div
          style={{
            fontSize: 12,
            color: "#94a3b8",
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
              border: "1px solid #d1d5db",
              borderRadius: 6,
              background: "#fff",
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
              background: newFileName.trim() ? "#6366F1" : "#d1d5db",
              color: "#fff",
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

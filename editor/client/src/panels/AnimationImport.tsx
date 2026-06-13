/**
 * 动画文件导入 — Lottie (.json) / Rive (.riv) 拖放区域。
 *
 * 拖入动画文件后自动生成 .omni 组件代码。
 * 支持 Lottie JSON 和 Rive 二进制文件。
 */

import React, { useState, useCallback } from "react";

// ── Types ─────────────────────────────────────────

interface AnimationImportProps {
  onImportLottie: (data: Record<string, unknown>, fileName: string) => void;
  onImportRive: (data: ArrayBuffer, fileName: string) => void;
  onInsertCode?: (code: string) => void;
}

type ImportState =
  | { status: "idle" }
  | { status: "dragover" }
  | { status: "processing"; fileName: string }
  | { status: "done"; fileName: string; code: string; type: "lottie" | "rive" }
  | { status: "error"; fileName: string; message: string };

// ── Code generation ───────────────────────────────

function generateLottieCode(
  fileName: string,
  _data: Record<string, unknown>
): string {
  const baseName = fileName.replace(/\.json$/i, "");
  const componentName = toPascalCase(baseName) + "Animation";

  return `// @editor: managed
component ${componentName} {
    signal src: String = "/animations/${fileName}";
    signal playing: bool = true;
    signal loop: bool = true;
    signal speed: double = 1.0;

    view {
        <LottiePlayer
            src={src}
            playing={playing}
            loop={loop}
            speed={speed}
        />
    }
}`;
}

function generateRiveCode(fileName: string): string {
  const baseName = fileName.replace(/\.riv$/i, "");
  const componentName = toPascalCase(baseName) + "Animation";

  return `// @editor: managed
component ${componentName} {
    signal src: String = "/animations/${fileName}";
    signal artboard: String = "default";
    signal stateMachine: String = "default";

    view {
        <RivePlayer
            src={src}
            artboard={artboard}
            stateMachine={stateMachine}
        />
    }
}`;
}

function toPascalCase(name: string): string {
  return name
    .replace(/[-_\s]+/g, " ")
    .split(" ")
    .map((w) => w.charAt(0).toUpperCase() + w.slice(1).toLowerCase())
    .join("");
}

// ── Component ─────────────────────────────────────

export function AnimationImport({
  onImportLottie,
  onImportRive,
  onInsertCode,
}: AnimationImportProps) {
  const [state, setState] = useState<ImportState>({ status: "idle" });

  const handleFile = useCallback(
    async (file: File) => {
      const fileName = file.name;

      try {
        setState({ status: "processing", fileName });

        if (fileName.endsWith(".json")) {
          const text = await file.text();
          const data = JSON.parse(text) as Record<string, unknown>;

          // 验证 Lottie JSON 格式
          if (!data.v && !data["__lottie_marker"]) {
            throw new Error("不是有效的 Lottie JSON（缺少 'v' 版本字段）");
          }

          onImportLottie(data, fileName);
          const code = generateLottieCode(fileName, data);
          setState({ status: "done", fileName, code, type: "lottie" });
        } else if (fileName.endsWith(".riv")) {
          const buffer = await file.arrayBuffer();
          onImportRive(buffer, fileName);
          const code = generateRiveCode(fileName);
          setState({ status: "done", fileName, code, type: "rive" });
        } else {
          throw new Error(
            `不支持的文件格式 "${fileName.split(".").pop()}"。请拖入 .json (Lottie) 或 .riv (Rive) 文件。`
          );
        }
      } catch (err) {
        const message = err instanceof Error ? err.message : "导入失败";
        setState({ status: "error", fileName, message });
      }
    },
    [onImportLottie, onImportRive]
  );

  const handleDrop = useCallback(
    (e: React.DragEvent) => {
      e.preventDefault();
      const file = e.dataTransfer.files[0];
      if (file) handleFile(file);
    },
    [handleFile]
  );

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    e.dataTransfer.dropEffect = "copy";
    setState({ status: "dragover" });
  }, []);

  const handleDragLeave = useCallback((e: React.DragEvent) => {
    // Only reset if actually leaving the drop zone (not entering a child)
    const rect = e.currentTarget.getBoundingClientRect();
    const { clientX: x, clientY: y } = e;
    if (x < rect.left || x > rect.right || y < rect.top || y > rect.bottom) {
      setState((prev) =>
        prev.status === "dragover" ? { status: "idle" } : prev
      );
    }
  }, []);

  const handleFileInput = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const file = e.target.files?.[0];
      if (file) handleFile(file);
    },
    [handleFile]
  );

  // ── Render ──────────────────────────────────────

  const isDragging = state.status === "dragover";
  const isProcessing = state.status === "processing";

  return (
    <div style={{ padding: 12 }}>
      {/* Header */}
      <div
        style={{
          fontSize: 11,
          fontWeight: 600,
          color: "#6B7280",
          textTransform: "uppercase",
          marginBottom: 8,
        }}
      >
        🎬 动画导入
      </div>

      {/* Drop zone */}
      <div
        onDrop={handleDrop}
        onDragOver={handleDragOver}
        onDragLeave={handleDragLeave}
        onClick={() => document.getElementById("anim-file-input")?.click()}
        style={{
          padding: 24,
          border: `2px dashed ${isDragging ? "#4F46E5" : "#D1D5DB"}`,
          borderRadius: 8,
          textAlign: "center" as const,
          color: isDragging ? "#4F46E5" : "#6B7280",
          fontSize: 13,
          background: isDragging ? "#EEF2FF" : "#FAFAFA",
          transition: "all 150ms ease",
          cursor: "pointer",
        }}
      >
        {isProcessing ? (
          <div>
            <div style={{ fontSize: 20, marginBottom: 4 }}>⏳</div>
            <div>处理中: {state.fileName}</div>
          </div>
        ) : (
          <div>
            <div style={{ fontSize: 28, marginBottom: 4 }}>
              {isDragging ? "📥" : "🎞️"}
            </div>
            <div>
              {isDragging
                ? "松手导入文件"
                : "拖入 Lottie (.json) 或 Rive (.riv) 文件"}
            </div>
            <div style={{ fontSize: 10, color: "#9CA3AF", marginTop: 4 }}>
              或点击选择文件
            </div>
          </div>
        )}
        <input
          id="anim-file-input"
          type="file"
          accept=".json,.riv"
          onChange={handleFileInput}
          style={{ display: "none" }}
        />
      </div>

      {/* Result */}
      {state.status === "done" && (
        <div
          style={{
            marginTop: 8,
            padding: 10,
            background: "#F0FDF4",
            border: "1px solid #A7F3D0",
            borderRadius: 8,
          }}
        >
          <div
            style={{
              display: "flex",
              justifyContent: "space-between",
              alignItems: "center",
              marginBottom: 6,
            }}
          >
            <span style={{ fontSize: 11, color: "#065F46", fontWeight: 600 }}>
              ✅ {state.type === "lottie" ? "Lottie" : "Rive"} 组件已生成
            </span>
            {onInsertCode && (
              <button
                onClick={() => {
                  onInsertCode(state.code);
                  setState({ status: "idle" });
                }}
                style={{
                  fontSize: 11,
                  padding: "3px 10px",
                  background: "#059669",
                  color: "#FFF",
                  border: "none",
                  borderRadius: 4,
                  cursor: "pointer",
                  fontWeight: 600,
                }}
              >
                插入代码
              </button>
            )}
          </div>
          <pre
            style={{
              margin: 0,
              fontSize: 10,
              maxHeight: 160,
              overflow: "auto",
              whiteSpace: "pre-wrap",
              color: "#374151",
              fontFamily: "monospace",
              background: "#FFF",
              padding: 8,
              borderRadius: 4,
            }}
          >
            {state.code}
          </pre>
        </div>
      )}

      {/* Error */}
      {state.status === "error" && (
        <div
          style={{
            marginTop: 8,
            padding: 8,
            background: "#FEF2F2",
            border: "1px solid #FECACA",
            borderRadius: 6,
            fontSize: 11,
            color: "#991B1B",
          }}
        >
          ❌ {state.message}
          <button
            onClick={() => setState({ status: "idle" })}
            style={{
              marginLeft: 8,
              background: "none",
              border: "none",
              color: "#991B1B",
              cursor: "pointer",
              textDecoration: "underline",
              fontSize: 11,
            }}
          >
            关闭
          </button>
        </div>
      )}

      {/* Help text */}
      <div
        style={{
          marginTop: 8,
          fontSize: 10,
          color: "#9CA3AF",
          lineHeight: 1.5,
        }}
      >
        支持格式：Lottie (.json)、Rive (.riv)。
        导入后自动生成 .omni 组件代码，插入到当前光标位置。
      </div>
    </div>
  );
}

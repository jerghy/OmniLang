/**
 * Read-only banner — shown when a hand-written .omni file is opened.
 * Displays warnings and offers conversion to visual project mode.
 */

import React, { useState } from "react";
import { api } from "../api/client";

interface ReadOnlyBannerProps {
  warnings: string[];
  source: string;
  onConverted?: () => void;
}

export function ReadOnlyBanner({ warnings, source, onConverted }: ReadOnlyBannerProps): React.ReactElement {
  const [converting, setConverting] = useState(false);
  const [error, setError] = useState("");

  const handleConvert = async () => {
    setConverting(true);
    setError("");
    try {
      const resp = await api.post("/api/omni-json/export", {
        body: JSON.stringify({
          source,
          filepath: "main.omni",
          target: "react",
        }),
      });
      if (resp.success && resp.document) {
        // Save the .omni.json alongside the .omni
        await api.post("/api/omni-json/save", {
          body: JSON.stringify({
            filePath: "project.omni.json",
            document: resp.document,
          }),
        });
        onConverted?.();
      } else {
        setError((resp as any).error || "Conversion failed");
      }
    } catch (e) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setConverting(false);
    }
  };

  return (
    <div
      style={{
        background: "linear-gradient(135deg, #FFF3CD 0%, #FFE69C 100%)",
        padding: "10px 16px",
        fontSize: 12,
        borderBottom: "1px solid #FFC107",
        display: "flex",
        alignItems: "flex-start",
        gap: 12,
        flexWrap: "wrap",
      }}
    >
      <div style={{ flex: 1, minWidth: 200 }}>
        <strong style={{ fontSize: 13 }}>🔒 Read-only Mode</strong>
        <span style={{ color: "#856404", marginLeft: 8 }}>
          This file was hand-written. The editor cannot write to it.
        </span>
        {warnings.length > 0 && (
          <ul style={{ margin: "4px 0 0 0", paddingLeft: 16, color: "#856404" }}>
            {warnings.map((w, i) => (
              <li key={i}>{w}</li>
            ))}
          </ul>
        )}
        {error && <div style={{ color: "#DC2626", marginTop: 4 }}>{error}</div>}
      </div>
      <button
        onClick={handleConvert}
        disabled={converting}
        style={{
          background: converting ? "#FCD34D" : "#F59E0B",
          color: "#FFF",
          border: "none",
          borderRadius: 6,
          padding: "6px 14px",
          fontSize: 12,
          fontWeight: 600,
          cursor: converting ? "not-allowed" : "pointer",
          whiteSpace: "nowrap",
        }}
      >
        {converting ? "Converting..." : "Convert to Visual Project"}
      </button>
    </div>
  );
}

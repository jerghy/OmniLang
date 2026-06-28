/**
 * 设计令牌面板 — 集中定义颜色、间距、字体等设计令牌。
 *
 * 令牌可被绑定到组件属性。修改令牌后，所有绑定的组件自动更新。
 * 令牌存储使用 Zustand。
 */
/* eslint-disable react-refresh/only-export-components -- intentional non-component exports */

import { useState, useCallback } from "react";
import { create } from "zustand";
import { contrastRatio, wcagLevel } from "../../a11y/ContrastChecker";

// ── Token Types ──────────────────────────────────

export interface DesignTokens {
  colors: Record<string, string>;
  spacing: Record<string, string>;
  typography: Record<string, string>;
  radii: Record<string, string>;
  shadows: Record<string, string>;
}

interface TokenStore {
  tokens: DesignTokens;
  setColor: (name: string, value: string) => void;
  removeColor: (name: string) => void;
  setSpacing: (name: string, value: string) => void;
  setTypography: (name: string, value: string) => void;
  setRadius: (name: string, value: string) => void;
}

export const useTokenStore = create<TokenStore>((set) => ({
  tokens: {
    colors: {
      primary: "#4F46E5",
      secondary: "#7C3AED",
      success: "#059669",
      warning: "#D97706",
      danger: "#DC2626",
      surface: "#FFFFFF",
      background: "#F9FAFB",
      textPrimary: "#111827",
      textSecondary: "#6B7280",
      border: "#E5E7EB",
    },
    spacing: {
      xs: "4px",
      sm: "8px",
      md: "16px",
      lg: "24px",
      xl: "32px",
      "2xl": "48px",
    },
    typography: {
      heading: "24px",
      body: "14px",
      caption: "12px",
      label: "11px",
    },
    radii: {
      none: "0",
      sm: "4px",
      md: "8px",
      lg: "12px",
      full: "9999px",
    },
    shadows: {
      none: "none",
      sm: "0 1px 2px rgba(0,0,0,0.05)",
      md: "0 4px 6px rgba(0,0,0,0.1)",
      lg: "0 10px 15px rgba(0,0,0,0.1)",
    },
  },
  setColor: (name, value) => set((s) => ({ tokens: { ...s.tokens, colors: { ...s.tokens.colors, [name]: value } } })),
  removeColor: (name) => set((s) => {
    const { [name]: _, ...rest } = s.tokens.colors;
    return { tokens: { ...s.tokens, colors: rest } };
  }),
  setSpacing: (name, value) => set((s) => ({ tokens: { ...s.tokens, spacing: { ...s.tokens.spacing, [name]: value } } })),
  setTypography: (name, value) => set((s) => ({ tokens: { ...s.tokens, typography: { ...s.tokens.typography, [name]: value } } })),
  setRadius: (name, value) => set((s) => ({ tokens: { ...s.tokens, radii: { ...s.tokens.radii, [name]: value } } })),
}));

// ── Panel Component ──────────────────────────────

type TokenCategory = "colors" | "spacing" | "typography" | "radii" | "shadows";

const CATEGORIES: { key: TokenCategory; label: string; icon: string }[] = [
  { key: "colors", label: "颜色", icon: "🎨" },
  { key: "spacing", label: "间距", icon: "📏" },
  { key: "typography", label: "字体", icon: "🔤" },
  { key: "radii", label: "圆角", icon: "⭕" },
  { key: "shadows", label: "阴影", icon: "🌑" },
];

export function TokenPanel() {
  const tokens = useTokenStore((s) => s.tokens);
  const setColor = useTokenStore((s) => s.setColor);
  const removeColor = useTokenStore((s) => s.removeColor);
  const setSpacing = useTokenStore((s) => s.setSpacing);
  const setTypography = useTokenStore((s) => s.setTypography);
  const setRadius = useTokenStore((s) => s.setRadius);

  const [activeCategory, setActiveCategory] = useState<TokenCategory>("colors");
  const [newName, setNewName] = useState("");
  const [newValue, setNewValue] = useState("");

  const currentTokens = tokens[activeCategory];

  const handleAdd = useCallback(() => {
    if (!newName.trim() || !newValue.trim()) return;
    switch (activeCategory) {
      case "colors": setColor(newName.trim(), newValue.trim()); break;
      case "spacing": setSpacing(newName.trim(), newValue.trim()); break;
      case "typography": setTypography(newName.trim(), newValue.trim()); break;
      case "radii": setRadius(newName.trim(), newValue.trim()); break;
    }
    setNewName(""); setNewValue("");
  }, [activeCategory, newName, newValue, setColor, setSpacing, setTypography, setRadius]);

  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", fontSize: 12 }}>
      {/* Header */}
      <div style={{ padding: "6px 8px", borderBottom: "1px solid #e5e7eb", fontWeight: 600, fontSize: 11, color: "#6B7280", textTransform: "uppercase" }}>
        🎨 设计令牌
      </div>

      {/* Category tabs */}
      <div style={{ display: "flex", borderBottom: "1px solid #f3f4f6" }}>
        {CATEGORIES.map((cat) => (
          <button
            key={cat.key}
            onClick={() => setActiveCategory(cat.key)}
            title={cat.label}
            style={{
              flex: 1, padding: "4px 2px", fontSize: 14, border: "none", cursor: "pointer",
              background: activeCategory === cat.key ? "#fff" : "transparent",
              borderBottom: activeCategory === cat.key ? "2px solid #4F46E5" : "2px solid transparent",
            }}
          >{cat.icon}</button>
        ))}
      </div>

      {/* Token list */}
      <div style={{ flex: 1, overflow: "auto", padding: "4px 8px" }}>
        {Object.entries(currentTokens).map(([name, value]) => (
          <div key={name} style={{ display: "flex", alignItems: "center", gap: 6, padding: "3px 0", borderBottom: "1px solid #f9fafb" }}>
            {activeCategory === "colors" && (
              <div style={{
                width: 18, height: 18, borderRadius: 4, border: "1px solid #e5e7eb",
                background: value as string, flexShrink: 0,
              }} />
            )}
            <span style={{ flex: 1, fontSize: 11, color: "#374151", fontWeight: 500 }}>{name}</span>
            <span style={{ fontSize: 10, color: "#9CA3AF", fontFamily: "monospace", maxWidth: 70, overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
              {value}
            </span>
            {activeCategory === "colors" && Object.keys(tokens.colors).length > 3 && (
              <button onClick={() => removeColor(name)}
                style={{ background: "none", border: "none", fontSize: 10, color: "#D1D5DB", cursor: "pointer", padding: 0 }}>✕</button>
            )}
          </div>
        ))}
      </div>

      {/* Add new token */}
      <div style={{ padding: "6px 8px", borderTop: "1px solid #e5e7eb" }}>
        <div style={{ display: "flex", gap: 4 }}>
          <input
            value={newName} onChange={(e) => setNewName(e.target.value)}
            placeholder="名称" onKeyDown={(e) => e.key === "Enter" && handleAdd()}
            style={{ flex: 1, padding: "3px 5px", fontSize: 10, border: "1px solid #e5e7eb", borderRadius: 3, minWidth: 0 }}
          />
          <input
            value={newValue} onChange={(e) => setNewValue(e.target.value)}
            placeholder={activeCategory === "colors" ? "#000000" : "16px"}
            onKeyDown={(e) => e.key === "Enter" && handleAdd()}
            style={{ width: 60, padding: "3px 5px", fontSize: 10, border: "1px solid #e5e7eb", borderRadius: 3 }}
          />
          <button onClick={handleAdd}
            style={{ padding: "3px 6px", fontSize: 10, background: "#4F46E5", color: "#fff", border: "none", borderRadius: 3, cursor: "pointer" }}>+</button>
        </div>
        {activeCategory === "colors" && (
          <div style={{ marginTop: 4 }}>
            <input
              type="color"
              value={newValue || "#000000"}
              onChange={(e) => setNewValue(e.target.value)}
              style={{ width: "100%", height: 24, border: "1px solid #e5e7eb", borderRadius: 3, cursor: "pointer", padding: 0 }}
            />
            {newValue && newValue.match(/^#[0-9a-fA-F]{6}$/) && (
              <ContrastBadgeInline color={newValue} />
            )}
          </div>
        )}
      </div>

      {/* Quick bind hint */}
      <div style={{ padding: "4px 8px", borderTop: "1px solid #f3f4f6", fontSize: 10, color: "#D1D5DB" }}>
        在属性面板中使用 --token-name 绑定
      </div>
    </div>
  );
}

// ── Contrast Badge (M5 a11y) ──────────────────────

function ContrastBadgeInline({ color }: { color: string }) {
  const onWhite = contrastRatio(color, "#FFFFFF");
  const onBlack = contrastRatio(color, "#111827");
  void wcagLevel(onWhite);
  void wcagLevel(onBlack);

  const bestBg = onWhite >= onBlack ? "#FFFFFF" : "#111827";
  const bestRatio = Math.max(onWhite, onBlack);
  const bestLevel = wcagLevel(bestRatio);

  const levelColors: Record<string, { bg: string; fg: string }> = {
    AAA: { bg: "#D1FAE5", fg: "#065F46" },
    AA: { bg: "#FEF3C7", fg: "#92400E" },
    FAIL: { bg: "#FEE2E2", fg: "#991B1B" },
  };

  const lc = levelColors[bestLevel];

  return (
    <div style={{ marginTop: 3, display: "flex", alignItems: "center", gap: 6, fontSize: 10 }}>
      <span
        style={{
          padding: "1px 6px", borderRadius: 3, fontWeight: 600,
          background: lc.bg, color: lc.fg,
        }}
      >
        {bestRatio.toFixed(1)}:1 {bestLevel === "AAA" ? "✅ AAA" : bestLevel === "AA" ? "⚠ AA" : "❌ FAIL"}
      </span>
      <span style={{ color: "#9CA3AF" }}>
        vs {bestBg === "#FFFFFF" ? "白底" : "黑底"}
      </span>
    </div>
  );
}

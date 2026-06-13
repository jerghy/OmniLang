/**
 * WCAG 2.1 对比度计算与检查。
 *
 * 在颜色选择器中嵌入 ContrastBadge 组件，实时显示
 * 前景色与背景色的对比度评分（AAA / AA / FAIL）。
 */

// ── 色彩数学 ───────────────────────────────────────

function hexToRgb(hex: string): { r: number; g: number; b: number } {
  const clean = hex.replace("#", "");
  if (clean.length === 3) {
    return {
      r: parseInt(clean[0] + clean[0], 16),
      g: parseInt(clean[1] + clean[1], 16),
      b: parseInt(clean[2] + clean[2], 16),
    };
  }
  return {
    r: parseInt(clean.slice(0, 2), 16),
    g: parseInt(clean.slice(2, 4), 16),
    b: parseInt(clean.slice(4, 6), 16),
  };
}

/** WCAG 2.1 相对亮度（sRGB）。 */
function relativeLuminance(r: number, g: number, b: number): number {
  const [rs, gs, bs] = [r, g, b].map((c) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4);
  });
  return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs;
}

/** 计算两个 hex 颜色之间的 WCAG 对比度。 */
export function contrastRatio(fg: string, bg: string): number {
  const fgRgb = hexToRgb(fg);
  const bgRgb = hexToRgb(bg);
  const l1 = relativeLuminance(fgRgb.r, fgRgb.g, fgRgb.b);
  const l2 = relativeLuminance(bgRgb.r, bgRgb.g, bgRgb.b);
  const lighter = Math.max(l1, l2);
  const darker = Math.min(l1, l2);
  return (lighter + 0.05) / (darker + 0.05);
}

export type WcagLevel = "AAA" | "AA" | "FAIL";

/** 根据 WCAG 2.1 阈值评估对比度级别。 */
export function wcagLevel(
  ratio: number,
  size: "normal" | "large" = "normal"
): WcagLevel {
  const aaThreshold = size === "large" ? 3.0 : 4.5;
  const aaaThreshold = size === "large" ? 4.5 : 7.0;
  if (ratio >= aaaThreshold) return "AAA";
  if (ratio >= aaThreshold) return "AA";
  return "FAIL";
}

// ── UI 组件 ────────────────────────────────────────

interface ContrastBadgeProps {
  fg: string;
  bg: string;
  /** 字体大小分类，large = ≥18px bold 或 ≥24px */
  size?: "normal" | "large";
}

/** 对比度评分徽章。嵌入颜色选择器旁使用。 */
export function renderContrastBadge(props: ContrastBadgeProps): {
  ratio: number;
  level: string;
  badge: React.CSSProperties;
  textStyle: React.CSSProperties;
  label: string;
} {
  const { fg, bg, size = "normal" } = props;
  const ratio = contrastRatio(fg, bg);
  const level = wcagLevel(ratio, size);

  const colors: Record<string, { bg: string; color: string }> = {
    AAA: { bg: "#D1FAE5", color: "#065F46" },
    AA: { bg: "#FEF3C7", color: "#92400E" },
    FAIL: { bg: "#FEE2E2", color: "#991B1B" },
  };

  const icons: Record<string, string> = {
    AAA: "✅",
    AA: "⚠️",
    FAIL: "❌",
  };

  return {
    ratio,
    level,
    badge: {
      display: "inline-flex",
      alignItems: "center",
      gap: 4,
      fontSize: 11,
      padding: "2px 8px",
      borderRadius: 4,
      fontWeight: 600,
      background: colors[level].bg,
      color: colors[level].color,
    },
    textStyle: {
      fontSize: 11,
      color: colors[level].color,
    },
    label: `${ratio.toFixed(1)}:1 ${icons[level]} ${level}`,
  };
}

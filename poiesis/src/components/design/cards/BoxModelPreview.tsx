// components/design/cards/BoxModelPreview.tsx
// Box-model preview diagram: margin (orange) → padding (green) → content (white).

interface SpacingValue {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

interface BoxModelPreviewProps {
  margin: SpacingValue;
  padding: SpacingValue;
  size?: number;
}

export function BoxModelPreview({
  margin,
  padding,
  size = 120,
}: BoxModelPreviewProps) {
  // Clamp each side so remaining content area stays positive
  const maxSide = Math.floor(size / 2 - 4);
  const clamp = (v: number) => Math.min(Math.max(v, 0), maxSide);

  const mTop = clamp(margin.top);
  const mRight = clamp(margin.right);
  const mBottom = clamp(margin.bottom);
  const mLeft = clamp(margin.left);

  const pTop = clamp(padding.top);
  const pRight = clamp(padding.right);
  const pBottom = clamp(padding.bottom);
  const pLeft = clamp(padding.left);

  // Padding box inset from the container edge
  const paddingBox = {
    top: mTop,
    right: mRight,
    bottom: mBottom,
    left: mLeft,
  };

  // Content box inset from the container edge
  const contentInset = {
    top: mTop + pTop,
    right: mRight + pRight,
    bottom: mBottom + pBottom,
    left: mLeft + pLeft,
  };

  const contentW = Math.max(size - contentInset.left - contentInset.right, 4);
  const contentH = Math.max(size - contentInset.top - contentInset.bottom, 4);

  return (
    <div
      style={{
        position: "relative",
        width: size,
        height: size,
        margin: "0 auto",
      }}
    >
      {/* Margin layer (outermost) */}
      <div
        style={{
          position: "absolute",
          inset: 0,
          background: "rgba(255, 152, 0, 0.25)",
          border: "1px solid rgba(255, 152, 0, 0.5)",
          borderRadius: 1,
        }}
      />

      {/* Padding layer (middle) */}
      <div
        style={{
          position: "absolute",
          top: paddingBox.top,
          right: paddingBox.right,
          bottom: paddingBox.bottom,
          left: paddingBox.left,
          background: "rgba(76, 175, 80, 0.2)",
          border: "1px solid rgba(76, 175, 80, 0.45)",
          borderRadius: 1,
        }}
      />

      {/* Content area (innermost) */}
      <div
        style={{
          position: "absolute",
          top: contentInset.top,
          left: contentInset.left,
          width: contentW,
          height: contentH,
          background: "#fff",
          border: "1px solid #b0bec5",
          borderRadius: 1,
        }}
      />

      {/* Margin top label */}
      {mTop > 0 && (
        <div
          style={{
            position: "absolute",
            top: 0,
            left: 0,
            right: 0,
            height: mTop,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 10,
            color: "#fff",
            fontWeight: 600,
            pointerEvents: "none",
          }}
        >
          M:{mTop}
        </div>
      )}

      {/* Margin bottom label */}
      {mBottom > 0 && (
        <div
          style={{
            position: "absolute",
            bottom: 0,
            left: 0,
            right: 0,
            height: mBottom,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 10,
            color: "#fff",
            fontWeight: 600,
            pointerEvents: "none",
          }}
        >
          M:{mBottom}
        </div>
      )}
    </div>
  );
}

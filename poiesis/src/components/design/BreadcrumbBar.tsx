// components/design/BreadcrumbBar.tsx
// Breadcrumb navigation bar at the bottom of the design page.

import { usePoiesisStore } from "../../store";

export function BreadcrumbBar() {
  const breadcrumbs = usePoiesisStore((s) => s.designPage.breadcrumbs);
  const navigateTo = usePoiesisStore((s) => s.navigateTo);
  const switchTab = usePoiesisStore((s) => s.switchTab);

  return (
    <div
      style={{
        display: "flex",
        height: 24,
        alignItems: "center",
        padding: "0 12px",
        background: "#f8fafc",
        borderTop: "1px solid #e2e8f0",
        fontSize: 11,
        gap: 4,
      }}
    >
      {breadcrumbs.map((crumb, i) => {
        const isLast = i === breadcrumbs.length - 1;
        return (
          <span
            key={crumb.id}
            style={{ display: "flex", alignItems: "center", gap: 4 }}
          >
            {i > 0 && <span style={{ color: "#cbd5e1" }}>›</span>}
            <span
              onClick={() => {
                if (crumb.type === "architecture") {
                  navigateTo("architecture");
                } else if (crumb.type === "component" && crumb.filePath) {
                  switchTab(crumb.id);
                }
              }}
              style={{
                color: isLast ? "#475569" : "#6366F1",
                cursor: isLast ? "default" : "pointer",
              }}
            >
              {crumb.label}
            </span>
          </span>
        );
      })}
      {breadcrumbs.length === 0 && (
        <span style={{ color: "#94a3b8" }}>架构</span>
      )}
    </div>
  );
}

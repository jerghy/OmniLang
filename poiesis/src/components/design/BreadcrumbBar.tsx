// components/design/BreadcrumbBar.tsx
// Breadcrumb navigation bar at the bottom of the design page.

import { usePoiesisStore } from "../../store";
import styles from "./BreadcrumbBar.module.css";

export function BreadcrumbBar() {
  const breadcrumbs = usePoiesisStore((s) => s.designPage.breadcrumbs);
  const navigateTo = usePoiesisStore((s) => s.navigateTo);
  const switchTab = usePoiesisStore((s) => s.switchTab);

  return (
    <div className={styles.bar}>
      {breadcrumbs.map((crumb, i) => {
        const isLast = i === breadcrumbs.length - 1;
        return (
          <span key={crumb.id} className={styles.crumbGroup}>
            {i > 0 && <span className={styles.separator}>›</span>}
            <span
              className={isLast ? styles.crumbLast : styles.crumbLink}
              role={isLast ? undefined : "button"}
              tabIndex={isLast ? undefined : 0}
              onClick={() => {
                if (crumb.type === "architecture") {
                  navigateTo("architecture");
                } else if (crumb.type === "component" && crumb.filePath) {
                  switchTab(crumb.id);
                }
              }}
              onKeyDown={(e) => {
                if (isLast) return;
                if (e.key === 'Enter' || e.key === ' ') {
                  e.preventDefault();
                  if (crumb.type === "architecture") {
                    navigateTo("architecture");
                  } else if (crumb.type === "component" && crumb.filePath) {
                    switchTab(crumb.id);
                  }
                }
              }}
            >
              {crumb.label}
            </span>
          </span>
        );
      })}
      {breadcrumbs.length === 0 && (
        <span className={styles.emptyMsg}>架构</span>
      )}
    </div>
  );
}

// components/design/TokenPickerPopover.tsx
// Popover for selecting a design token to bind to a property.

import { forwardRef } from "react";
import type { DesignToken } from "../../core/types/design";
import styles from "./TokenPickerPopover.module.css";

const CATEGORIES = ["colors", "spacing", "typography", "radii"];

interface TokenPickerPopoverProps {
  tokens: DesignToken[];
  onSelectToken: (tokenName: string) => void;
  onUnlinkToken: () => void;
}

export const TokenPickerPopover = forwardRef<HTMLDivElement, TokenPickerPopoverProps>(
  function TokenPickerPopover({ tokens, onSelectToken, onUnlinkToken }, ref) {
    if (tokens.length === 0) {
      return (
        <div ref={ref} className={styles.popover}>
          <div className={styles.sectionLabel}>
            Link Token
          </div>
          <div className={styles.emptyText}>
            No tokens defined
          </div>
        </div>
      );
    }

    const categories = CATEGORIES;

    return (
      <div ref={ref} className={styles.popover}>
        <div className={styles.sectionLabel}>
          Link Token
        </div>
        {categories.map((cat) => {
          const catTokens = tokens.filter((t) => t.category === cat);
          if (catTokens.length === 0) return null;
          return (
            <div key={cat}>
              <div className={styles.catLabel}>
                {cat}
              </div>
              {catTokens.map((t) => (
                <button type="button"
                  key={t.id}
                  onClick={() => onSelectToken(t.name)}
                  className={styles.tokenBtn}
                  onMouseEnter={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "var(--dp-accent-subtle, #eef0ff)";
                  }}
                  onMouseLeave={(e) => {
                    (e.currentTarget as HTMLElement).style.background = "transparent";
                  }}
                >
                  {t.tokenType === "color" && (
                    <span
                      className={styles.colorSwatch}
                      style={{ background: t.value }}
                    />
                  )}
                  <span>{t.name}</span>
                  <span className={styles.tokenValue}>
                    {t.value}
                  </span>
                </button>
              ))}
            </div>
          );
        })}
        <div className={styles.unlinkSection}>
          <button type="button"
            onClick={onUnlinkToken}
            className={styles.unlinkBtn}
            onMouseEnter={(e) => {
              (e.currentTarget as HTMLElement).style.background = "#fef2f2";
            }}
            onMouseLeave={(e) => {
              (e.currentTarget as HTMLElement).style.background = "transparent";
            }}
          >
            Unlink token
          </button>
        </div>
      </div>
    );
  },
);

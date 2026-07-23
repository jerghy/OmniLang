// components/design/NewTokenMenu.tsx
// Dropdown menu for selecting a token category when creating a new token.

import { forwardRef } from "react";
import styles from "./NewTokenMenu.module.css";

const CATEGORY_ORDER = ["colors", "spacing", "typography", "radii"] as const;

const CATEGORY_ICONS: Record<string, string> = {
  colors: "🎨",
  spacing: "📏",
  typography: "🔤",
  radii: "⬜",
};

const CATEGORY_LABELS: Record<string, string> = {
  colors: "Colors",
  spacing: "Spacing",
  typography: "Typography",
  radii: "Radii",
};

interface NewTokenMenuProps {
  onSelect: (category: string) => void;
}

export const NewTokenMenu = forwardRef<HTMLDivElement, NewTokenMenuProps>(
  function NewTokenMenu({ onSelect }, ref) {
    return (
      <div
        ref={(el) => {
          if (typeof ref === 'function') ref(el);
          else if (ref) (ref as React.MutableRefObject<HTMLDivElement | null>).current = el;
          if (el) {
            const btn = el.previousSibling as HTMLElement;
            if (btn) {
              const r = btn.getBoundingClientRect();
              el.style.top = `${r.bottom + 4}px`;
              el.style.left = `${r.right - 160}px`;
            }
          }
        }}
        className={styles.menu}
      >
        {CATEGORY_ORDER.map((cat) => (
          <button type="button"
            key={cat}
            onClick={() => onSelect(cat)}
            className={styles.menuItem}
            onMouseEnter={(e) => { (e.currentTarget as HTMLElement).style.background = "var(--ol-gray-100)"; }}
            onMouseLeave={(e) => { (e.currentTarget as HTMLElement).style.background = "transparent"; }}
          >
            <span>{CATEGORY_ICONS[cat]}</span>
            <span>{CATEGORY_LABELS[cat]}</span>
          </button>
        ))}
      </div>
    );
  },
);

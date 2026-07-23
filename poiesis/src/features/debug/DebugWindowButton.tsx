import { useDebugWindow } from "./useDebugWindow";
import styles from "./DebugWindowButton.module.css";

export function DebugWindowButton() {
  const { isOpen, open, close } = useDebugWindow();

  return (
    <button type="button"
      onClick={isOpen ? close : open}
      title={isOpen ? "Close debug window" : "Open debug window"}
      className={styles.button}
      style={{
        background: isOpen ? "#6366f1" : "transparent",
        color: isOpen ? "#fff" : "inherit",
      }}
    >
      🐛 {isOpen ? "Debug On" : "Debug"}
    </button>
  );
}

import { useDebugWindow } from "./useDebugWindow";

export function DebugWindowButton() {
  const { isOpen, open, close } = useDebugWindow();

  return (
    <button type="button"
      onClick={isOpen ? close : open}
      title={isOpen ? "Close debug window" : "Open debug window"}
      style={{
        background: isOpen ? "#6366f1" : "transparent",
        color: isOpen ? "#fff" : "inherit",
        border: "1px solid #d4d4d8",
        borderRadius: 6,
        padding: "4px 10px",
        cursor: "pointer",
        fontSize: 14,
        display: "flex",
        alignItems: "center",
        gap: 4,
      }}
    >
      🐛 {isOpen ? "Debug On" : "Debug"}
    </button>
  );
}

// components/design/ConfirmCloseDialog.tsx
// Confirmation dialog shown when closing a dirty tab.

import styles from "./ConfirmCloseDialog.module.css";

interface ConfirmCloseDialogProps {
  onConfirm: () => void;
  onCancel: () => void;
}

export function ConfirmCloseDialog({
  onConfirm,
  onCancel,
}: ConfirmCloseDialogProps) {
  return (
    <>
      <button
        type="button"
        tabIndex={0}
        aria-label="Close confirmation"
        onClick={onCancel}
        onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.preventDefault(); onCancel(); } }}
        className={styles.backdrop}
      />
      <div
        className={styles.dialog}
      >
        <div className={styles.dialogTitle}>
          未保存的更改
        </div>
        <div className={styles.dialogText}>
          此标签页有未保存的更改。确定要关闭吗？
        </div>
        <div className={styles.dialogActions}>
          <button type="button"
            onClick={onCancel}
            className={styles.cancelBtn}
          >
            取消
          </button>
          <button type="button"
            onClick={onConfirm}
            className={styles.confirmBtn}
          >
            关闭
          </button>
        </div>
      </div>
    </>
  );
}

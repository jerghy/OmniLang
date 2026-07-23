// poiesis/src/components/canvas/code/cards/card-types.ts
import type { CodeOp } from "../../../../core/types/code-op";

export interface CardComponentProps<TOp extends CodeOp = CodeOp> {
  op: TOp;
  isSelected: boolean;
  onUpdate: (updated: TOp) => void;
  onDelete: () => void;
  /** Called when user presses Enter at the card's right boundary to insert a new line after */
  onInsertAfter: () => void;
  /** Called when cursor navigates to a nested body (if/when/for) */
  onEnterBody?: () => void;
}

export interface CardListProps {
  ops: CodeOp[];
  selectedPid: string | null;
  onSelect: (pid: string) => void;
  onUpdate: (index: number, updated: CodeOp) => void;
  onDelete: (index: number) => void;
  onInsertAfter: (index: number) => void;
}

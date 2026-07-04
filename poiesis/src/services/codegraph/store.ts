// Legacy adapter: codegraph store merged into unified PoiesisStore.
import { usePoiesisStore } from "../../store";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const useCodeGraphStore = usePoiesisStore as any;
export type CodeEditorState = Record<string, unknown>;

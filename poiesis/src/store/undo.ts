// Legacy adapter: re-export the unified store as useUndoStore.
import { usePoiesisStore } from "./index";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const useUndoStore = usePoiesisStore as any;

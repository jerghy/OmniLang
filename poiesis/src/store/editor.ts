// Legacy adapter: re-export the unified store as useEditorStore
// so old code continues to compile during migration.
import { usePoiesisStore } from "./index";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const useEditorStore = usePoiesisStore as any;
export type { ElementData } from "../core/types/element";
export type { WireData } from "../core/types/wire";

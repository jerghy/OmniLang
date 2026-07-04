// Legacy component registry — usePoiesisStore handles component defs.
import { usePoiesisStore } from "../../store";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const useComponentRegistry = usePoiesisStore as any;
export type ComponentDefinition = Record<string, unknown>;

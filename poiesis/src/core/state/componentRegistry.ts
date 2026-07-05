// Legacy component registry — usePoiesisStore handles component defs.
import { usePoiesisStore } from "../../store";
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const useComponentRegistry = usePoiesisStore as any;
export type ComponentDefinition = Record<string, unknown>;

/** Safe accessor for component definitions — handles the componentDefs vs definitions mismatch. */
export function getComponentDefinitions(): Record<string, ComponentDefinition> {
  const state = useComponentRegistry.getState();
  return (state?.componentDefs ?? state?.definitions ?? {}) as Record<string, ComponentDefinition>;
}

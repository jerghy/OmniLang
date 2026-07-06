// Legacy component registry — usePoiesisStore handles component defs.
import { usePoiesisStore } from "../../store";
import type { PoiesisState } from "../../store";

/** Extended store state with legacy definitions field. */
interface ExtendedState extends PoiesisState {
  definitions?: Record<string, Record<string, unknown>>;
}

// Re-export the unified store since component defs are now on PoiesisState.
export const useComponentRegistry = usePoiesisStore as unknown as {
  <T>(selector?: (state: ExtendedState) => T): T;
  getState(): ExtendedState;
  setState(partial: ExtendedState | Partial<ExtendedState>): void;
  subscribe(listener: (state: ExtendedState, prevState: ExtendedState) => void): () => void;
  destroy(): void;
};

export type ComponentDefinition = Record<string, unknown>;

/** Safe accessor for component definitions — handles the componentDefs vs definitions mismatch. */
export function getComponentDefinitions(): Record<string, ComponentDefinition> {
  const state = usePoiesisStore.getState() as ExtendedState;
  return (state?.componentDefs ?? state?.definitions ?? {}) as unknown as Record<string, ComponentDefinition>;
}

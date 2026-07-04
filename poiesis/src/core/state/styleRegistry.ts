// Legacy style registry stub — merged into unified store.
import { usePoiesisStore } from "../../store";

export const styleRegistry = {
  classes: {},
  cssVars: {},
};

// Hybrid: callable as a hook + has .getState()/.setState()/.subscribe()
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type StoreFn = { (selector?: (state: any) => unknown): any; getState(): any; setState(p: any): void; subscribe(fn?: (s: any) => unknown): () => void; destroy(): void };

const _getState = () => usePoiesisStore.getState() as any;

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const styleStore: StoreFn = ((selector?: (state: any) => unknown): any => {
  const store = _getState();
  return selector ? selector(store) : store;
}) as StoreFn;

styleStore.getState = _getState;
styleStore.setState = (partial: any) => usePoiesisStore.setState(partial);
styleStore.subscribe = (selector?: (s: any) => unknown) => {
  return usePoiesisStore.subscribe((s) => selector?.(s as any));
};
styleStore.destroy = () => {};

export const useStyleRegistryStore = styleStore;

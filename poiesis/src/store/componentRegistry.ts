// Legacy adapter: re-export store functions used by ComponentPalette, ComponentTree, etc.
import { usePoiesisStore } from "./index";
import type { PoiesisState } from "./index";
import { COMPONENT_PROTOCOL } from "../data/componentProtocol";
import type { ComponentProtocolEntry } from "../data/componentProtocol";
import type { ComponentDefinition } from "../core/state/componentRegistry";

/** Component registry store interface — extends PoiesisState with legacy `definitions` field. */
interface ComponentRegistryState extends PoiesisState {
  definitions?: Record<string, ComponentDefinition>;
}

// Re-export the unified store — the `definitions` field is set at runtime
// via useComponentRegistry.setState({ definitions: ... }).
export const useComponentRegistry = usePoiesisStore as unknown as {
  <T>(selector?: (state: ComponentRegistryState) => T): T;
  getState(): ComponentRegistryState;
  setState(partial: ComponentRegistryState | Partial<ComponentRegistryState>): void;
  subscribe(listener: (state: ComponentRegistryState, prevState: ComponentRegistryState) => void): () => void;
  destroy(): void;
};

/** Safe accessor — handles the componentDefs vs definitions mismatch. */
export function getComponentDefinitions(): Record<string, ComponentDefinition> {
  const state = usePoiesisStore.getState() as ComponentRegistryState;
  return (state?.componentDefs ?? state?.definitions ?? {}) as unknown as Record<string, ComponentDefinition>;
}

/** Return unified component metadata (built-in + custom). */
export function getUnifiedComponentMeta(): Record<string, ComponentProtocolEntry> {
  const state = usePoiesisStore.getState();
  const customDefs = state.componentDefs ?? {};

  // Guard against module initialization order issues where COMPONENT_PROTOCOL
  // may not be fully initialized when this function is called during render.
  const unified: Record<string, ComponentProtocolEntry> = COMPONENT_PROTOCOL
    ? { ...COMPONENT_PROTOCOL }
    : {};

  for (const [name, def] of Object.entries(customDefs)) {
    const typedDef = def as unknown as Record<string, unknown>;
    unified[name] = {
      component: name,
      category: (typedDef.category as string) || "Custom",
      description: (typedDef.description as string) || "",
      props: {},
      events: {},
      styleProperties: (typedDef.styleProperties as string[]) || [],
      children: (typedDef.children as boolean) ?? true,
    };
  }

  return unified;
}

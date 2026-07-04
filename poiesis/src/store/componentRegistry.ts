// Legacy adapter: re-export store functions used by ComponentPalette, ComponentTree, etc.
import { usePoiesisStore } from "./index";
import { COMPONENT_PROTOCOL } from "../data/componentProtocol";
import type { ComponentProtocolEntry } from "../data/componentProtocol";

// Typed as any so legacy code accessing non-existent properties compiles
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export const useComponentRegistry = usePoiesisStore as any;

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

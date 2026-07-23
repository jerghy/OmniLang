import { useShallow } from "zustand/react/shallow";
import { usePoiesisStore } from "./index";
import type { ElementData } from "../core/types/element";
import type { WireData } from "../core/types/wire";

// Stable reference selector for elements
export function useElements(): ElementData[] {
  return usePoiesisStore(useShallow((s) => s.elements));
}

export function useWires(): WireData[] {
  return usePoiesisStore(useShallow((s) => s.wires));
}

// FlowCanvas-specific — only codeblock elements
export function useCodeElements(): ElementData[] {
  return usePoiesisStore(
    useShallow((s) => s.elements.filter((e) => e.nodeType === "codeblock"))
  );
}

export function useRootOrder(): string[] {
  return usePoiesisStore(useShallow((s) => s.rootOrder));
}

export function useSelectedElementIds(): string[] {
  return usePoiesisStore(useShallow((s) => s.selectedElementIds));
}

export function useComponentDefs() {
  return usePoiesisStore(useShallow((s) => s.componentDefs));
}

function useActiveTabId(): string | null {
  return usePoiesisStore((s) => s.designPage.activeTabId);
}

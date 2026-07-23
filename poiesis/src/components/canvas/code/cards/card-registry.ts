// poiesis/src/components/canvas/code/cards/card-registry.ts
import type { CodeOp } from "../../../../core/types/code-op";
import type { FC } from "react";
import type { CardComponentProps } from "./card-types";

// Lazily populated — card components register themselves
const registry: Record<string, FC<CardComponentProps<any>>> = {};

export function registerCard(kind: CodeOp["kind"], component: FC<CardComponentProps<any>>): void {
  registry[kind] = component;
}

export function getCardComponent(kind: CodeOp["kind"]): FC<CardComponentProps<any>> | undefined {
  return registry[kind];
}

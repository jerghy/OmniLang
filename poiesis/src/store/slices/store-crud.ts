// ── Store CRUD slice for Poiesis store ──

import type { PoiesisState } from "../types";
import type { StoreDefinition, StoreSignalDef, StoreGetterDef, StoreActionDef } from "../../core/types/store";
import type { PidEntry } from "../../core/types/pid";

type SetFn = (partial: Partial<PoiesisState> | ((state: PoiesisState) => Partial<PoiesisState>)) => void;
type GetFn = () => PoiesisState;

export function createStoreCrudSlice(set: SetFn, _get?: GetFn) {
  return {
    storeDefs: {} as Record<string, StoreDefinition>,
    storeReferences: {} as Record<string, string[]>,

    loadStoreDef: (def: StoreDefinition) => set((s) => ({
      storeDefs: { ...s.storeDefs, [def.name]: def },
    })),

    removeStoreDef: (name: string) => set((s) => {
      const { [name]: _removed, ...rest } = s.storeDefs;
      return { storeDefs: rest };
    }),

    // ── Signal CRUD ──
    addStoreSignal: (storeName: string, signal: StoreSignalDef) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: { ...store, signals: [...store.signals, signal] },
        },
        isDirty: true,
      };
    }),

    updateStoreSignal: (storeName: string, pid: string, updates: Partial<StoreSignalDef>) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            signals: store.signals.map((sig) =>
              sig.pid === pid ? { ...sig, ...updates } : sig
            ),
          },
        },
        isDirty: true,
      };
    }),

    removeStoreSignal: (storeName: string, pid: string) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            signals: store.signals.filter((sig) => sig.pid !== pid),
          },
        },
        isDirty: true,
      };
    }),

    // ── Getter CRUD ──
    addStoreGetter: (storeName: string, getter: StoreGetterDef) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: { ...store, getters: [...store.getters, getter] },
        },
        isDirty: true,
      };
    }),

    updateStoreGetter: (storeName: string, pid: string, updates: Partial<StoreGetterDef>) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            getters: store.getters.map((g) =>
              g.pid === pid ? { ...g, ...updates } : g
            ),
          },
        },
        isDirty: true,
      };
    }),

    removeStoreGetter: (storeName: string, pid: string) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            getters: store.getters.filter((g) => g.pid !== pid),
          },
        },
        isDirty: true,
      };
    }),

    // ── Action CRUD ──
    addStoreAction: (storeName: string, action: StoreActionDef) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: { ...store, actions: [...store.actions, action] },
        },
        isDirty: true,
      };
    }),

    updateStoreAction: (storeName: string, pid: string, updates: Partial<StoreActionDef>) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            actions: store.actions.map((a) =>
              a.pid === pid ? { ...a, ...updates } : a
            ),
          },
        },
        isDirty: true,
      };
    }),

    removeStoreAction: (storeName: string, pid: string) => set((s) => {
      const store = s.storeDefs[storeName];
      if (!store) return s;
      return {
        storeDefs: {
          ...s.storeDefs,
          [storeName]: {
            ...store,
            actions: store.actions.filter((a) => a.pid !== pid),
          },
        },
        isDirty: true,
      };
    }),

    // ── PID index registration ──
    registerStorePids: (storeDef: StoreDefinition) => set((s) => {
      const entries: Record<string, PidEntry> = {};
      const filePath = storeDef.filePath;
      entries[storeDef.pid] = { pid: storeDef.pid, blockType: "store", blockName: storeDef.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      for (const sig of storeDef.signals) {
        entries[sig.pid] = { pid: sig.pid, blockType: "signal", blockName: sig.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      }
      for (const g of storeDef.getters) {
        entries[g.pid] = { pid: g.pid, blockType: "getter", blockName: g.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      }
      for (const a of storeDef.actions) {
        entries[a.pid] = { pid: a.pid, blockType: "action", blockName: a.name, sourceFile: filePath, startLine: 0, endLine: 0 };
      }
      return { pidIndex: { ...s.pidIndex, ...entries } };
    }),

    // ── @store reference tracking ──
    registerStoreRef: (componentName: string, storeName: string) => set((s) => {
      const existing = s.storeReferences[storeName] || [];
      if (existing.includes(componentName)) return s;
      return { storeReferences: { ...s.storeReferences, [storeName]: [...existing, componentName] } };
    }),

    unregisterStoreRef: (componentName: string, storeName: string) => set((s) => {
      const existing = s.storeReferences[storeName];
      if (!existing) return s;
      return {
        storeReferences: {
          ...s.storeReferences,
          [storeName]: existing.filter((c) => c !== componentName),
        },
      };
    }),

    unregisterAllStoreRefs: (componentName: string) => set((s) => {
      const updated: Record<string, string[]> = {};
      let changed = false;
      for (const [storeName, refs] of Object.entries(s.storeReferences)) {
        const filtered = refs.filter((c) => c !== componentName);
        if (filtered.length !== refs.length) changed = true;
        updated[storeName] = filtered;
      }
      return changed ? { storeReferences: updated } : s;
    }),
  };
}

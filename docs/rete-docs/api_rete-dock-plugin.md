# API for rete-dock-plugin package

> Source: [https://retejs.org/docs/api/rete-dock-plugin](https://retejs.org/docs/api/rete-dock-plugin)

# API for rete-dock-plugin package

## DockPlugin

[index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/index.ts#L16)

Dock plugin. Allows to create nodes from the dock by dragging or clicking.

```
tsclass DockPlugin<Schemes extends BaseSchemes>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

**Extends** Scope<never, Area2DInherited<Schemes, never>>

**Emits** `render`

### add

[index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/index.ts#L41)

Add node to the dock.

```
tsadd(create: () => Schemes["Node"], index: number): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| create | () => Schemes["Node"] | Function that creates node. |
| index | number | Index of the node in the dock, optional. |

**Returns** `void`

### addPreset

[index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/index.ts#L81)

Add preset to the dock plugin.

```
tsaddPreset(preset: Preset): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | Preset | Preset that will manage dock items. |

**Returns** `void`

### remove

[index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/index.ts#L69)

Remove node from the dock.

```
tsremove(create: () => Schemes["Node"]): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| create | () => Schemes["Node"] | Function that creates node. Must be the same as in add method. |

**Returns** `void`

## DockPresets

[presets/index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/presets/index.ts#L1)

Presets for the dock plugin

### classic

[presets/classic/index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/presets/classic/index.ts#L1)

#### setup

[presets/classic/index.ts](https://github.com/retejs/dock-plugin/blob/a904b97/src/presets/classic/index.ts#L63)

Classic preset for the dock plugin.
Creates container for list of nodes and containers each node.

```
tssetup(props: { area: AreaPlugin<BaseSchemes, T>, scale: number, size: number }): Preset

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { area: AreaPlugin<BaseSchemes, T>, scale: number, size: number } | Preset properties |

**Returns** `Preset`

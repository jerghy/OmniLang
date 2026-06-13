# API for @retejs/lit-plugin package

> Source: [https://retejs.org/docs/api/rete-lit-plugin](https://retejs.org/docs/api/rete-lit-plugin)

# API for @retejs/lit-plugin package

## LitPlugin

[index.tsx](https://github.com/retejs/lit-plugin/blob/469c798/src/index.tsx#L31)

React plugin. Renders nodes, connections and other elements using React.

```
tsclass LitPlugin<Schemes extends BaseSchemes, T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| T | unknown |  |

**Extends** Scope<Produces<Schemes>, Requires<Schemes> | T>

**Emits** `connectionpath`

**Listens** `render` `unmount`

### addPreset

[index.tsx](https://github.com/retejs/lit-plugin/blob/469c798/src/index.tsx#L117)

Adds a preset to the plugin.

```
tsaddPreset(preset: RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible">): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible"> | Preset that can render nodes, connections and other elements. |

**Returns** `void`

## Produces

[index.tsx](https://github.com/retejs/lit-plugin/blob/469c798/src/index.tsx#L16)

Signals that can be emitted by the plugin

```
tstype Produces<Schemes extends BaseSchemes> = {
  data: Object;
  type: "connectionpath";
};

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

## Presets

[presets/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/index.ts#L1)

Built-in presets, responsible for rendering different parts of the editor.

### classic

[presets/classic/index.tsx](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/classic/index.tsx#L1)

#### setup

[presets/classic/index.tsx](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/classic/index.tsx#L41)

Classic preset for rendering nodes, connections, controls and sockets.

```
tssetup(props: ClassicProps<Schemes, K>): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | ClassicProps<Schemes, K> |  |

**Returns** `RenderPreset<Schemes, K>`

### contextMenu

[presets/context-menu/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/context-menu/index.ts#L1)

#### setup

[presets/context-menu/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/context-menu/index.ts#L14)

Preset for rendering context menu.

```
tssetup(props: { delay: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { delay: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### minimap

[presets/minimap/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/minimap/index.ts#L1)

#### setup

[presets/minimap/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/minimap/index.ts#L13)

Preset for rendering minimap.

```
tssetup(props: { size: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { size: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### reroute

[presets/reroute/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/reroute/index.ts#L1)

#### setup

[presets/reroute/index.ts](https://github.com/retejs/lit-plugin/blob/469c798/src/presets/reroute/index.ts#L19)

Preset for rendering pins.

```
tssetup(props: Props): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props |  |

**Returns** `RenderPreset<Schemes, K>`

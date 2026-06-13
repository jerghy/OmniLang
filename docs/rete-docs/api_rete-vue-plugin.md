# API for rete-vue-plugin package

> Source: [https://retejs.org/docs/api/rete-vue-plugin](https://retejs.org/docs/api/rete-vue-plugin)

# API for rete-vue-plugin package

## Produces

[index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/index.ts#L17)

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

## VuePlugin

[index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/index.ts#L44)

Vue plugin. Renders nodes, connections and other elements using React.

```
tsclass VuePlugin<Schemes extends BaseSchemes, T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| T | unknown |  |

**Extends** Scope<Produces<Schemes>, Requires<Schemes> | T>

**Emits** `connectionpath`

**Listens** `render` `unmount`

### addPreset

[index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/index.ts#L127)

Adds a preset to the plugin.

```
tsaddPreset(preset: RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible">): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible"> | Preset that can render nodes, connections and other elements. |

**Returns** `void`

## Presets

[presets/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/index.ts#L1)

Built-in presets, responsible for rendering different parts of the editor.

### classic

[presets/classic/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/classic/index.ts#L1)

#### setup

[presets/classic/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/classic/index.ts#L37)

Classic preset for rendering nodes, connections, controls and sockets.

```
tssetup(props: ClassicProps<Schemes, K>): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | ClassicProps<Schemes, K> |  |

**Returns** `RenderPreset<Schemes, K>`

### contextMenu

[presets/context-menu/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/context-menu/index.ts#L1)

#### setup

[presets/context-menu/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/context-menu/index.ts#L10)

Preset for rendering context menu.

```
tssetup(props: { delay: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { delay: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### minimap

[presets/minimap/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/minimap/index.ts#L1)

#### setup

[presets/minimap/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/minimap/index.ts#L10)

Preset for rendering minimap.

```
tssetup(props: { size: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { size: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### reroute

[presets/reroute/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/reroute/index.ts#L1)

#### setup

[presets/reroute/index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/presets/reroute/index.ts#L17)

Preset for rendering pins.

```
tssetup(props: Props): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props |  |

**Returns** `RenderPreset<Schemes, K>`

## Props

[index.ts](https://github.com/retejs/vue-plugin/blob/a1ae872/src/index.ts#L28)

Vue plugin options used to setup vue instance(s) used by retejs.

```
tstype Props = { setup: Function };

```

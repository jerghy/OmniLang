# API for rete-react-plugin package

> Source: [https://retejs.org/docs/api/rete-react-plugin](https://retejs.org/docs/api/rete-react-plugin)

# API for rete-react-plugin package

## ReactPlugin

[src/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/index.tsx#L43)

React plugin. Renders nodes, connections and other elements using React.

```
tsclass ReactPlugin<Schemes extends BaseSchemes, T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| T | unknown |  |

**Extends** Scope<Produces<Schemes>, Requires<Schemes> | T>

**Emits** `connectionpath`

**Listens** `render` `unmount`

### addPreset

[src/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/index.tsx#L109)

Adds a preset to the plugin.

```
tsaddPreset(preset: RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible">): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible"> | Preset that can render nodes, connections and other elements. |

**Returns** `void`

## Produces

[src/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/index.tsx#L20)

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

[src/presets/index.ts](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/index.ts#L1)

Built-in presets, responsible for rendering different parts of the editor.

### classic

[src/presets/classic/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/classic/index.tsx#L1)

#### setup

[src/presets/classic/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/classic/index.tsx#L46)

Classic preset for rendering nodes, connections, controls and sockets.

```
tssetup(props: ClassicProps<Schemes, K>): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | ClassicProps<Schemes, K> |  |

**Returns** `RenderPreset<Schemes, K>`

### contextMenu

[src/presets/context-menu/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/context-menu/index.tsx#L1)

#### setup

[src/presets/context-menu/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/context-menu/index.tsx#L22)

Preset for rendering context menu.

```
tssetup(props: Props): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props |  |

**Returns** `RenderPreset<Schemes, K>`

### minimap

[src/presets/minimap/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/minimap/index.tsx#L1)

#### setup

[src/presets/minimap/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/minimap/index.tsx#L12)

Preset for rendering minimap.

```
tssetup(props: { size: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { size: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### reroute

[src/presets/reroute-pins/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/reroute-pins/index.tsx#L1)

#### setup

[src/presets/reroute-pins/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/presets/reroute-pins/index.tsx#L20)

Preset for rendering pins.

```
tssetup(props: Props): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props |  |

**Returns** `RenderPreset<Schemes, K>`

## Props

[src/index.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/index.tsx#L31)

Plugin props

```
tstype Props = { createRoot: Function };

```

## RefComponent

[src/ref-component.tsx](https://github.com/retejs/react-plugin/blob/7ac40f0/src/ref-component.tsx#L9)

Component for rendering various elements embedded in the React.js component tree.

```
tsRefComponent(__namedParameters: Props): Element

```

| Parameter | Type | Description |
| --- | --- | --- |
| __namedParameters | Props |  |

**Returns** `Element`

# API for rete-angular-plugin package

> Source: [https://retejs.org/docs/api/rete-angular-plugin](https://retejs.org/docs/api/rete-angular-plugin)

# API for rete-angular-plugin package

## Produces

[src/core.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/core.ts#L61)

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

## AngularPlugin

[src/core.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/core.ts#L76)

Angular plugin. Renders nodes, connections and other elements using React.

```
tsclass AngularPlugin<Schemes extends BaseSchemes, T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| T | unknown |  |

**Extends** Scope<Produces<Schemes>, Requires<Schemes> | T>

**Emits** `connectionpath`

**Listens** `render` `unmount`

### constructor

[src/core.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/core.ts#L86)

```
tsconstructor(params: { injector: Injector }): AngularPlugin<Schemes, T>

```

| Parameter | Type | Description |
| --- | --- | --- |
| params | { injector: Injector } | Plugin properties |

**Returns** `AngularPlugin<Schemes, T>`

### addPreset

[src/core.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/core.ts#L160)

Adds a preset to the plugin.

```
tsaddPreset(preset: RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible">): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| preset | RenderPreset<Schemes, CanAssignEachTupleElemmentToAnyOf<T, UnionToTuple<K>> extends true ? K : "Cannot apply preset. Provided signals are not compatible"> | Preset that can render nodes, connections and other elements. |

**Returns** `void`

## Presets

[src/presets/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/index.ts#L1)

Built-in presets, responsible for rendering different parts of the editor.

### classic

[src/presets/classic/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/classic/index.ts#L1)

#### setup

[src/presets/classic/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/classic/index.ts#L29)

Classic preset for rendering nodes, connections, controls and sockets.

```
tssetup(props: ClassicProps<Schemes, K>): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | ClassicProps<Schemes, K> |  |

**Returns** `RenderPreset<Schemes, K>`

### contextMenu

[src/presets/context-menu/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/context-menu/index.ts#L1)

#### setup

[src/presets/context-menu/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/context-menu/index.ts#L10)

Preset for rendering context menu.

```
tssetup(props: { delay: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { delay: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### minimap

[src/presets/minimap/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/minimap/index.ts#L1)

#### setup

[src/presets/minimap/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/minimap/index.ts#L10)

Preset for rendering minimap.

```
tssetup(props: { size: number }): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { size: number } |  |

**Returns** `RenderPreset<Schemes, K>`

### reroute

[src/presets/reroute/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/reroute/index.ts#L1)

#### setup

[src/presets/reroute/index.ts](https://github.com/retejs/angular-plugin/blob/f6fca36/src/presets/reroute/index.ts#L17)

Preset for rendering pins.

```
tssetup(props: Props): RenderPreset<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props |  |

**Returns** `RenderPreset<Schemes, K>`

# API for rete-minimap-plugin package

> Source: [https://retejs.org/docs/api/rete-minimap-plugin](https://retejs.org/docs/api/rete-minimap-plugin)

# API for rete-minimap-plugin package

## MinimapExtra

[index.ts](https://github.com/retejs/minimap-plugin/blob/3c4ca92/src/index.ts#L22)

Extra signal types for minimap rendering

```
tstype MinimapExtra = RenderSignal<
  "minimap",
  {
    nodes: Rect[];
    ratio: number;
    viewport: Rect;
    point: unknown;
    start: unknown;
    translate: unknown;
  }
>;

```

## MinimapPlugin

[index.ts](https://github.com/retejs/minimap-plugin/blob/3c4ca92/src/index.ts#L44)

Minimap plugin, triggers rendering of the minimap

```
tsclass MinimapPlugin<Schemes extends ExpectedScheme>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ExpectedScheme |  |

**Extends** Scope<never, Area2D<Schemes> | MinimapExtra, Root<Schemes>>

**Emits** `render`

**Listens** `nodetranslated` `nodecreated` `noderemoved` `translated` `resized` `noderesized` `zoomed`

### constructor

[index.ts](https://github.com/retejs/minimap-plugin/blob/3c4ca92/src/index.ts#L60)

```
tsconstructor(props: { boundViewport: boolean, minDistance: number, ratio: number }): MinimapPlugin<Schemes>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | { boundViewport: boolean, minDistance: number, ratio: number } | Plugin properties |

**Returns** `MinimapPlugin<Schemes>`

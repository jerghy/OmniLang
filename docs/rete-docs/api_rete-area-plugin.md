# API for rete-area-plugin package

> Source: [https://retejs.org/docs/api/rete-area-plugin](https://retejs.org/docs/api/rete-area-plugin)

# API for rete-area-plugin package

## BaseArea

[src/base.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/base.ts#L34)

A union of all possible signals that can be emitted by any area plugin

```
tstype BaseArea<Schemes extends BaseSchemes> =
  | { data: Object; type: "nodepicked" }
  | { data: Schemes["Node"]; type: "nodedragged" }
  | { data: Object & NodeTranslateEventParams; type: "nodetranslate" }
  | { data: Object & NodeTranslateEventParams; type: "nodetranslated" }
  | { data: Object; type: "contextmenu" }
  | { data: Object; type: "pointerdown" }
  | { data: Object; type: "pointermove" }
  | { data: Object; type: "pointerup" }
  | { data: Object & NodeResizeEventParams; type: "noderesize" }
  | { data: Object & NodeResizeEventParams; type: "noderesized" }
  | RenderSignal<"node", { payload: Schemes["Node"] }>
  | RenderSignal<
      "connection",
      { end: Position; payload: Schemes["Connection"]; start: Position }
    >
  | { data: Object; type: "unmount" }
  | { data: Object; type: "reordered" };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

## Area2D

[src/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/index.ts#L25)

A union of all possible signals that can be emitted by the area

```
tstype Area2D<Schemes extends BaseSchemes> =
  | BaseArea<Schemes>
  | { data: TranslateEventParams; type: "translate" }
  | { data: TranslateEventParams; type: "translated" }
  | { data: ZoomEventParams; type: "zoom" }
  | { data: ZoomEventParams; type: "zoomed" }
  | { data: Object; type: "resized" };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |

## AreaPlugin

[src/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/index.ts#L46)

A plugin that provides a 2D area for nodes and connections

```
tsclass AreaPlugin<Schemes extends BaseSchemes, ExtraSignals extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| ExtraSignals | unknown |  |

**Extends** BaseAreaPlugin<Schemes, Area2D<Schemes> | ExtraSignals>

**Emits** `render` `rendered` `unmount`

**Listens** `nodecreated` `noderemoved` `connectioncreated` `connectionremoved`

### destroy

[src/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/index.ts#L204)

Destroy all views and remove all event listeners

```
tsdestroy(): void

```

**Returns** `void`

### resize

[src/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/index.ts#L184)

Resize node

```
tsresize(id: string, width: number, height: number): Promise<undefined | boolean>

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Node id |
| width | number | Desired width |
| height | number | Desired height |

**Returns** `Promise<undefined | boolean>`

### translate

[src/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/index.ts#L195)

Translate node to position

```
tstranslate(id: string, position: Position): Promise<undefined | boolean>

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Node id |
| position | Position | Position |

**Returns** `Promise<undefined | boolean>`

### update

[src/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/index.ts#L172)

Force update rendered element by id (node, connection, etc.)

```
tsupdate(type: "node" | "connection" | GetRenderTypes<ExtraSignals>, id: string): Promise<void>

```

**Emits** `render`

| Parameter | Type | Description |
| --- | --- | --- |
| type | `"node" | "connection" |
| id | string | Element id |

**Returns** `Promise<void>`

## AreaExtensions

[src/extensions/index.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/index.ts#L1)

Area extensions

### Selector

[src/extensions/selectable.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/selectable.ts#L40)

Selector class. Used to collect selected entities (nodes, connections, etc.) and synchronize them (select, unselect, translate, etc.).
Can be extended to add custom functionality.

```
tsclass Selector<E extends SelectorEntity>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| E | SelectorEntity |  |

### Restrictor

[src/extensions/restrictor.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/restrictor.ts#L12)

Restrictor extension parameters

```
tstype Restrictor = {
  scaling: ScaleRange | Function | boolean;
  translation: TranslateRange | Function | boolean;
};

```

### SnapGrid

[src/extensions/snap.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/snap.ts#L8)

Snap grid extension parameters

```
tstype SnapGrid = { dynamic: boolean; size: number };

```

### ZoomAt

[src/extensions/zoom-at.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/zoom-at.ts#L11)

Zoom extension parameters

```
tstype ZoomAt = { scale: number };

```

### accumulateOnCtrl

[src/extensions/selectable.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/selectable.ts#L10)

Selector's accumulate function, activated when the ctrl key is pressed

```
tsaccumulateOnCtrl(): { active: unknown, destroy: unknown }

```

**Returns** `{ active: unknown, destroy: unknown }`

### getBoundingBox

[src/extensions/bounding-box.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/bounding-box.ts#L14)

Get the bounding box of the given nodes

```
tsgetBoundingBox(plugin: BaseAreaPlugin<Schemes, K>, nodes: NodeRef<Schemes>[]): { bottom: number, center: Object, height: number, left: number, right: number, top: number, width: number }

```

| Parameter | Type | Description |
| --- | --- | --- |
| plugin | BaseAreaPlugin<Schemes, K> | The area plugin |
| nodes | NodeRef<Schemes>[] | The nodes to get the bounding box of |

**Returns** `{ bottom: number, center: Object, height: number, left: number, right: number, top: number, width: number }`
The bounding box

### restrictor

[src/extensions/restrictor.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/restrictor.ts#L27)

Restrictor extension. Restricts the area zoom and position

```
tsrestrictor(plugin: AreaPlugin<Schemes, K>, params: Restrictor): void

```

**Listens** `zoom` `zoomed` `translated`

| Parameter | Type | Description |
| --- | --- | --- |
| plugin | AreaPlugin<Schemes, K> | The area plugin |
| params | Restrictor | The restrictor parameters |

**Returns** `void`

### selectableNodes

[src/extensions/selectable.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/selectable.ts#L112)

Selectable nodes extension. Adds the ability to select nodes in the area.

```
tsselectableNodes(base: BaseAreaPlugin<Schemes, T>, core: Selector<SelectorEntity>, options: { accumulating: Accumulating }): { select: Function, unselect: Function }

```

**Listens** `nodepicked` `nodetranslated` `pointerdown` `pointermove` `pointerup`

| Parameter | Type | Description |
| --- | --- | --- |
| base | BaseAreaPlugin<Schemes, T> | BaseAreaPlugin instance |
| core | Selector<SelectorEntity> | Selectable instance |
| options | { accumulating: Accumulating } |  |

**Returns** `{ select: Function, unselect: Function }`

### selector

[src/extensions/selectable.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/selectable.ts#L88)

Selector factory, uses default Selector class

```
tsselector(): Selector<E>

```

**Returns** `Selector<E>`
Selector instance

### showInputControl

[src/extensions/show-input-control.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/show-input-control.ts#L15)

Show input control extension. It will show the input's control when there is no connection and hide it when there is a connection.

```
tsshowInputControl(area: BaseAreaPlugin<BaseSchemes, any>, visible: Visible<S>): void

```

**Listens** `connectioncreated` `connectionremoved`

| Parameter | Type | Description |
| --- | --- | --- |
| area | BaseAreaPlugin<BaseSchemes, any> | The base area plugin |
| visible | Visible<S> | The visible function |

**Returns** `void`

### simpleNodesOrder

[src/extensions/order.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/order.ts#L11)

Simple nodes order extension

```
tssimpleNodesOrder(base: BaseAreaPlugin<Schemes, T>): void

```

**Listens** `nodepicked` `connectioncreated`

| Parameter | Type | Description |
| --- | --- | --- |
| base | BaseAreaPlugin<Schemes, T> | The base area plugin |

**Returns** `void`

### snapGrid

[src/extensions/snap.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/snap.ts#L22)

Snap grid extension

```
tssnapGrid(base: BaseAreaPlugin<Schemes, K>, params: SnapGrid): void

```

**Listens** `nodetranslate` `nodedragged`

| Parameter | Type | Description |
| --- | --- | --- |
| base | BaseAreaPlugin<Schemes, K> | The base area plugin |
| params | SnapGrid | The snap parameters |

**Returns** `void`

### zoomAt

[src/extensions/zoom-at.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/extensions/zoom-at.ts#L23)

Zooms the area to fit the given nodes

```
tszoomAt(plugin: AreaPlugin<Schemes, K>, nodes: NodeRef<Schemes>[], params: ZoomAt): Promise<void>

```

| Parameter | Type | Description |
| --- | --- | --- |
| plugin | AreaPlugin<Schemes, K> | The area plugin |
| nodes | NodeRef<Schemes>[] | The nodes to fit |
| params | ZoomAt | The zoom parameters |

**Returns** `Promise<void>`

## BaseAreaPlugin

[src/base.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/base.ts#L54)

Base abstract class for area plugins that provides a common interface

```
tsclass BaseAreaPlugin<Schemes extends BaseSchemes, Signals extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| Signals | unknown |  |

**Extended by** AreaPlugin

**Extends** Scope<Signals, Root<Schemes>>

## Drag

[src/drag.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/drag.ts#L23)

Drag handler, used to handle dragging of the area and nodes. Can be extended to add custom behavior.

```
tsclass Drag

```

## Zoom

[src/zoom.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/zoom.ts#L12)

Zoom class, used to handle zooming of the area. Can be extended to add custom behavior.

```
tsclass Zoom

```

## usePointerListener

[src/utils.ts](https://github.com/retejs/area-plugin/blob/f862e2e/src/utils.ts#L16)

listen to pointerdown, window's pointermove and pointerup events,
where last two not active before pointerdown triggered for performance reasons

```
tsusePointerListener(element: HTMLElement, handlers: PointerListenerHandlers): PointerListener

```

| Parameter | Type | Description |
| --- | --- | --- |
| element | HTMLElement |  |
| handlers | PointerListenerHandlers |  |

**Returns** `PointerListener`

# API for rete-render-utils package

> Source: [https://retejs.org/docs/api/rete-render-utils](https://retejs.org/docs/api/rete-render-utils)

# API for rete-render-utils package

## BaseSocketPosition

[sockets-position/base-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/base-socket-position.ts#L24)

Abstract class for socket position calculation. It can be extended to implement custom socket position calculation.

```
tsclass BaseSocketPosition<Schemes extends BaseSchemes, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| K | unknown |  |

**Extended by** DOMSocketPosition

**Implements** SocketPositionWatcher<Scope<never, K>>

**Listens** `render` `rendered` `unmount` `nodetranslated` `noderesized`

### attach

[sockets-position/base-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/base-socket-position.ts#L42)

Attach the watcher to the area's child scope.

```
tsattach(scope: Scope<never, [K]>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| scope | Scope<never, [K]> | Scope of the watcher that should be a child of BaseAreaPlugin |

**Returns** `void`

### calculatePosition

[sockets-position/base-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/base-socket-position.ts#L36)

The method needs to be implemented that calculates the position of the socket.

```
tscalculatePosition(nodeId: string, side: Side, key: string, element: HTMLElement): Promise<null | Position>

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node ID |
| side | Side | Side of the socket, 'input' or 'output' |
| key | string | Socket key |
| element | HTMLElement | Socket element |

**Returns** `Promise<null | Position>`

### listen

[sockets-position/base-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/base-socket-position.ts#L94)

Listen to socket position changes. Usually used by rendering plugins to update the start/end of the connection.

```
tslisten(nodeId: string, side: Side, key: string, change: OnChange): () => void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node ID |
| side | Side | Side of the socket, 'input' or 'output' |
| key | string | Socket key |
| change | OnChange | Callback function that is called when the socket position changes |

**Returns** `() => void`

## DOMSocketPosition

[sockets-position/dom-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/dom-socket-position.ts#L25)

Class for socket position calculation based on DOM elements. It uses `getElementCenter` function to calculate the position.

```
tsclass DOMSocketPosition<Schemes extends BaseSchemes, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | BaseSchemes |  |
| K | unknown |  |

**Extends** BaseSocketPosition<Schemes, K>

### attach

[sockets-position/base-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/base-socket-position.ts#L42)

Attach the watcher to the area's child scope.

```
tsattach(scope: Scope<never, [K]>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| scope | Scope<never, [K]> | Scope of the watcher that should be a child of BaseAreaPlugin |

**Returns** `void`

### listen

[sockets-position/base-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/base-socket-position.ts#L94)

Listen to socket position changes. Usually used by rendering plugins to update the start/end of the connection.

```
tslisten(nodeId: string, side: Side, key: string, change: OnChange): () => void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node ID |
| side | Side | Side of the socket, 'input' or 'output' |
| key | string | Socket key |
| change | OnChange | Callback function that is called when the socket position changes |

**Returns** `() => void`

## SocketPositionWatcher

[sockets-position/types.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/types.ts#L10)

Interface for socket position watcher.

```
tstype SocketPositionWatcher<ChildScope extends unknown> = {
  attach: unknown;
  listen: unknown;
};

```

| Parameter | Extends | Description |
| --- | --- | --- |
| ChildScope | unknown |  |

**Implemented by** BaseSocketPosition

## classicConnectionPath

[connection.ts](https://github.com/retejs/render-utils/blob/f24f374/src/connection.ts#L8)

Get classic SVG path for a connection between two points.

```
tsclassicConnectionPath(points: [Position, Position], curvature: number): string

```

| Parameter | Type | Description |
| --- | --- | --- |
| points | [Position, Position] | Array of two points |
| curvature | number | Curvature of the connection |

**Returns** `string`

## getDOMSocketPosition

[sockets-position/dom-socket-position.ts](https://github.com/retejs/render-utils/blob/f24f374/src/sockets-position/dom-socket-position.ts#L49)

Wrapper function for `DOMSocketPosition` class.

```
tsgetDOMSocketPosition(props: Props): DOMSocketPosition<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props | Props for DOMSocketPosition class |

**Returns** `DOMSocketPosition<Schemes, K>`

## getElementCenter

[utils.ts](https://github.com/retejs/render-utils/blob/f24f374/src/utils.ts#L11)

Calculates the center coordinates of a child element relative to a parent element.

```
tsgetElementCenter(child: HTMLElement, parent: HTMLElement): Promise<{ x: number, y: number }>

```

**Throws** Error if the child element has a null offsetParent.

| Parameter | Type | Description |
| --- | --- | --- |
| child | HTMLElement | The child element whose center coordinates need to be calculated. |
| parent | HTMLElement | The parent element relative to which the child element's center is calculated. |

**Returns** `Promise<{ x: number, y: number }>`
Position of the child element's center

## loopConnectionPath

[connection.ts](https://github.com/retejs/render-utils/blob/f24f374/src/connection.ts#L23)

Get loop SVG path for a connection between two points.

```
tsloopConnectionPath(points: [Position, Position], curvature: number, size: number): string

```

| Parameter | Type | Description |
| --- | --- | --- |
| points | [Position, Position] | Array of two points |
| curvature | number | Curvature of the loop |
| size | number | Size of the loop |

**Returns** `string`

# API for rete-area-3d-plugin package

> Source: [https://retejs.org/docs/api/rete-area-3d-plugin](https://retejs.org/docs/api/rete-area-3d-plugin)

# API for rete-area-3d-plugin package

## Area3D

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L21)

Signal types produced by Area3DPlugin instance

```
tstype Area3D<Schemes extends ExpectSchemes> =
  | BaseArea<Schemes>
  | { data: TransformEventParams; type: "transform" }
  | { data: TransformEventParams; type: "transformed" }
  | { data: Object; type: "resized" };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ExpectSchemes |  |

## Area3DPlugin

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L33)

Plugin for embedding node editor into 3D scene.

```
tsclass Area3DPlugin<Schemes extends ExpectSchemes, ExtraSignals extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ExpectSchemes |  |
| ExtraSignals | unknown |  |

**Extends** BaseAreaPlugin<Schemes, Area3D<Schemes> | ExtraSignals>

### constructor

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L46)

```
tsconstructor(container: HTMLElement): Area3DPlugin<Schemes, ExtraSignals>

```

| Parameter | Type | Description |
| --- | --- | --- |
| container | HTMLElement | HTML element to render area in |

**Returns** `Area3DPlugin<Schemes, ExtraSignals>`

### area

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L37)

Area instance, contains nodes, connections and other elements

```
tsarea: Area<Area3DPlugin<Schemes, ExtraSignals>>;

```

### destroy

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L213)

Destroy all views and remove all event listeners

```
tsdestroy(): void

```

**Returns** `void`

### resize

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L192)

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

### share

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L77)

Share the 3D scene with multiple instances of Area3DPlugin

```
tsshare(): Area3DPlugin<Schemes, ExtraSignals>

```

**Returns** `Area3DPlugin<Schemes, ExtraSignals>`
new instance of Area3DPlugin

### translate

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L180)

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

[index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/index.ts#L204)

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

## Area3DExtensions

[extensions/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/index.ts#L1)

Area 3D extensions.
These extensions are specific to the 3D area and provide additional functionality, while many of rete-area-plugin's extensions are also available for this plugin.

### forms

[extensions/forms/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/index.ts#L1)

Group of functions to replicate forms.
This extension provides a functions for generating geometry that replicates the form of HTML elements embedded in the scene.

#### ClassicNodeOptions

[extensions/forms/node/geometry.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/node/geometry.ts#L10)

Classic node geometry parameters

```
tstype ClassicNodeOptions = {
  borderRadius: number;
  inputsOffset: number;
  outputsOffset: number;
  socketMargin: number;
  socketRadius: number;
};

```

#### ConnectionForm

[extensions/forms/connection/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/connection/index.ts#L18)

Options for node form generator

```
tstype ConnectionForm = { customize: Function };

```

#### NodeForm

[extensions/forms/node/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/node/index.ts#L13)

Options for node form generator

```
tstype NodeForm<S extends ExpectSchemes> = { customize: Function };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | ExpectSchemes |  |

#### comment

[extensions/forms/comment.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/comment.ts#L40)

Form generator for embedding rete-comment-plugin into 3D scene.

```
tscomment(scope: ScopeAsParameter<Scope<K, any[]>, [Requires]>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| scope | ScopeAsParameter<Scope<K, any[]>, [Requires]> | Area3DPlugin instance |

**Returns** `void`

#### connection

[extensions/forms/connection/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/connection/index.ts#L28)

Form generator for embedding connections into 3D scene.

```
tsconnection(scope: ScopeAsParameter<Scope<K, E[]>, [{ data: Object, type: "connectionpath" }]>, props: ConnectionForm): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| scope | ScopeAsParameter<Scope<K, E[]>, [{ data: Object, type: "connectionpath" }]> | Area3DPlugin instance |
| props | ConnectionForm | Options for connection form generator |

**Returns** `void`

#### createClassicConnectionGeometry

[extensions/forms/connection/geometry.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/connection/geometry.ts#L16)

Create geometry for classic connection.
Can be used in `customize` option.

```
tscreateClassicConnectionGeometry(path: string, width: number): BufferGeometry

```

| Parameter | Type | Description |
| --- | --- | --- |
| path | string | SVG path |
| width | number | Connection width |

**Returns** `BufferGeometry`
Connection geometry

#### createClassicNodeGeometry

[extensions/forms/node/geometry.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/node/geometry.ts#L30)

Create classic node geometry

```
tscreateClassicNodeGeometry(size: Size, params: ClassicNodeOptions): BufferGeometry

```

| Parameter | Type | Description |
| --- | --- | --- |
| size | Size | Node size |
| params | ClassicNodeOptions | Geometry parameters |

**Returns** `BufferGeometry`
Node geometry

#### node

[extensions/forms/node/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/node/index.ts#L23)

Form generator for embedding nodes into 3D scene.

```
tsnode(area: Area3DPlugin<S, K>, props: NodeForm<S>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| area | Area3DPlugin<S, K> | Area3DPlugin instance |
| props | NodeForm<S> | Options for node form generator |

**Returns** `void`

#### reroute

[extensions/forms/reroute.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/forms/reroute.ts#L26)

Form generator for embedding rete-connection-reroute-plugin into 3D scene.

```
tsreroute(scope: ScopeAsParameter<Scope<K, [Root<S>]>, [RenderSignal<"reroute-pins", { data: PinData }>]>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| scope | ScopeAsParameter<Scope<K, [Root<S>]>, [RenderSignal<"reroute-pins", { data: PinData }>]> | Area3DPlugin instance |

**Returns** `void`

### LookAt

[extensions/lookAt.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/lookAt.ts#L10)

Parameters for `zoomAt` extension

```
tstype LookAt = { scale: number };

```

### animate

[extensions/animate.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/animate.ts#L11)

Animate the given 3D scene, uses `requestAnimationFrame`

```
tsanimate(area: Area3DPlugin<S, K>, tick: (time: number) => void): void

```

**Examples**

```
tsArea3DExtensions.animate(area)

```

```
tsArea3DExtensions.animate(area, time => console.log(time))

```

| Parameter | Type | Description |
| --- | --- | --- |
| area | Area3DPlugin<S, K> | The 3D area plugin |
| tick | (time: number) => void | Optional callback to be called on each frame |

**Returns** `void`

### lookAt

[extensions/lookAt.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/extensions/lookAt.ts#L23)

Move the camera to look at the given nodes

```
tslookAt(area: Area3DPlugin<S, K>, nodes: S["Node"][], params: LookAt): void

```

**Examples**

```
tsArea3DExtensions.lookAt(area, [node1, node2])

```

```
tsArea3DExtensions.lookAt(area, [node1, node2], { scale: 0.8 })

```

| Parameter | Type | Description |
| --- | --- | --- |
| area | Area3DPlugin<S, K> | The 3D area plugin |
| nodes | S["Node"][] | The nodes to look at |
| params | LookAt | The lookAt parameters |

**Returns** `void`

## Area

[area.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/area.ts#L25)

Area class is responsible for managing 3D scene for a current editor

```
tsclass Area<Scope extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Scope | unknown |  |

### getCanvas

[area.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/area.ts#L53)

Get the canvas in form of Object3D for the current scope

```
tsgetCanvas(): undefined | Object3D<Object3DEventMap>

```

**Returns** `undefined | Object3D<Object3DEventMap>`

### resize

[area.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/area.ts#L90)

Force resize the container of the 3D scene

```
tsresize(event: Event): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| event | Event | The resize event |

**Returns** `void`

### setPointerFrom

[area.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/area.ts#L60)

Set the pointer position from the given mouse event into the `pointer` property

```
tssetPointerFrom(event: MouseEvent): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| event | MouseEvent |  |

**Returns** `void`

## HybridRenderer

[scene/HybridRenderer.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/scene/HybridRenderer.ts#L7)

HybridRenderer is a wrapper for Three.js CSS3DRenderer and WebGLRenderer.

```
tsclass HybridRenderer

```

### css3d

[scene/HybridRenderer.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/scene/HybridRenderer.ts#L9)

CSS3DRenderer instance created by default

```
tscss3d: CSS3DRenderer;

```

### webgl

[scene/HybridRenderer.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/scene/HybridRenderer.ts#L11)

WebGLRenderer instance created by default

```
tswebgl: WebGLRenderer;

```

## HybridScene

[scene/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/scene/index.ts#L21)

HybridScene is a wrapper for Three.js Scene with some additional features such as:

- drag controls
- orbit controls
- camera
- renderer

```
tsclass HybridScene<Scope extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Scope | unknown |  |

### render

[scene/index.ts](https://github.com/retejs/area-3d-plugin/blob/dbce796/src/scene/index.ts#L115)

Render the frame. Can be directly used instead of `animate` extension.

```
tsrender(): void

```

**Returns** `void`

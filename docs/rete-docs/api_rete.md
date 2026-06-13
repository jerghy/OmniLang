# API for rete package

> Source: [https://retejs.org/docs/api/rete](https://retejs.org/docs/api/rete)

# API for rete package

## Root

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L10)

Signal types produced by NodeEditor instance

```
tstype Root<Scheme extends BaseSchemes> =
  | { data: Scheme["Node"]; type: "nodecreate" }
  | { data: Scheme["Node"]; type: "nodecreated" }
  | { data: Scheme["Node"]; type: "noderemove" }
  | { data: Scheme["Node"]; type: "noderemoved" }
  | { data: Scheme["Connection"]; type: "connectioncreate" }
  | { data: Scheme["Connection"]; type: "connectioncreated" }
  | { data: Scheme["Connection"]; type: "connectionremove" }
  | { data: Scheme["Connection"]; type: "connectionremoved" }
  | { type: "clear" }
  | { type: "clearcancelled" }
  | { type: "cleared" };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Scheme | BaseSchemes | The scheme type |

## NodeEditor

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L29)

The NodeEditor class is the entry class. It is used to create and manage nodes and connections.

```
tsclass NodeEditor<Scheme extends BaseSchemes>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Scheme | BaseSchemes | The scheme type |

**Extends** Scope<Root<Scheme>>

### addConnection

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L98)

Add a connection

```
tsaddConnection(data: Scheme["Connection"]): Promise<boolean>

```

**Emits** `connectioncreate` `connectioncreated`

**Throws** If the connection has already been added

| Parameter | Type | Description |
| --- | --- | --- |
| data | Scheme["Connection"] | The connection data |

**Returns** `Promise<boolean>`
Whether the connection was added

### addNode

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L79)

Add a node

```
tsaddNode(data: Scheme["Node"]): Promise<boolean>

```

**Emits** `nodecreate` `nodecreated`

**Throws** If the node has already been added

| Parameter | Type | Description |
| --- | --- | --- |
| data | Scheme["Node"] | The node data |

**Returns** `Promise<boolean>`
Whether the node was added

### clear

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L160)

Clear all nodes and connections

```
tsclear(): Promise<boolean>

```

**Emits** `clear` `clearcancelled` `cleared`

**Returns** `Promise<boolean>`
Whether the editor was cleared

### getConnection

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L67)

Get a connection by id

```
tsgetConnection(id: Scheme["Connection"]["id"]): Scheme["Connection"]

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | Scheme["Connection"]["id"] | The connection id |

**Returns** `Scheme["Connection"]`
The connection or undefined

### getConnections

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L58)

Get all connections

```
tsgetConnections(): Scheme["Connection"][]

```

**Returns** `Scheme["Connection"][]`
Copy of array with onnections

### getNode

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L42)

Get a node by id

```
tsgetNode(id: Scheme["Node"]["id"]): Scheme["Node"]

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | Scheme["Node"]["id"] | The node id |

**Returns** `Scheme["Node"]`
The node or undefined

### getNodes

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L50)

Get all nodes

```
tsgetNodes(): Scheme["Node"][]

```

**Returns** `Scheme["Node"][]`
Copy of array with nodes

### removeConnection

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L139)

Remove a connection

```
tsremoveConnection(id: Scheme["Connection"]["id"]): Promise<boolean>

```

**Emits** `connectionremove` `connectionremoved`

**Throws** If the connection cannot be found

| Parameter | Type | Description |
| --- | --- | --- |
| id | Scheme["Connection"]["id"] | The connection id |

**Returns** `Promise<boolean>`
Whether the connection was removed

### removeNode

[editor.ts](https://github.com/retejs/rete/blob/baa7397/src/editor.ts#L117)

Remove a node

```
tsremoveNode(id: Scheme["Node"]["id"]): Promise<boolean>

```

**Emits** `noderemove` `noderemoved`

**Throws** If the node cannot be found

| Parameter | Type | Description |
| --- | --- | --- |
| id | Scheme["Node"]["id"] | The node id |

**Returns** `Promise<boolean>`
Whether the node was removed

## ClassicPreset

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L1)

Contains classes for classic scheme such as Node, Input, Output, Control, Socket, Connection

### Node

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L173)

The node class

```
tsclass Node<Inputs extends { [key in string]?: Socket }, Outputs extends { [key in string]?: Socket }, Controls extends { [key in string]?: Control }>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Inputs | { [key in string]?: Socket } |  |
| Outputs | { [key in string]?: Socket } |  |
| Controls | { [key in string]?: Control } |  |

**Implements** NodeBase

**Examples**

```
tsnew Node('math')

```

#### controls

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L193)

Node controls

```
tscontrols: Controls;

```

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L181)

Node id, unique string generated by `getUID` function

```
tsid: string;

```

#### inputs

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L185)

Node inputs

```
tsinputs: { [key in string | number | symbol]?: Input<Exclude<Inputs[key], undefined>> }

```

#### outputs

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L189)

Node outputs

```
tsoutputs: { [key in string | number | symbol]?: Output<Exclude<Outputs[key], undefined>> }

```

#### selected

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L197)

Whether the node is selected. Default is `false`

```
tsselected: boolean;

```

### Connection

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L250)

The connection class

```
tsclass Connection<Source extends Node, Target extends Node>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Source | Node |  |
| Target | Node |  |

**Implements** ConnectionBase

#### constructor

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L274)

```
tsconstructor(source: Source, sourceOutput: keyof Source["outputs"], target: Target, targetInput: keyof Target["inputs"]): Connection<Source, Target>

```

| Parameter | Type | Description |
| --- | --- | --- |
| source | Source | Source node instance |
| sourceOutput | keyof Source["outputs"] | Source node output key |
| target | Target | Target node instance |
| targetInput | keyof Target["inputs"] | Target node input key |

**Returns** `Connection<Source, Target>`

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L257)

Connection id, unique string generated by `getUID` function

```
tsid: string;

```

#### source

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L261)

Source node id

```
tssource: string;

```

#### sourceOutput

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L276)

Source node output key

```
tssourceOutput: keyof Source["outputs"]

```

#### target

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L265)

Target node id

```
tstarget: string;

```

#### targetInput

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L278)

Target node input key

```
tstargetInput: keyof Target["inputs"]

```

### Socket

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L16)

The socket class

```
tsclass Socket

```

#### constructor

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L21)

```
tsconstructor(name: string): Socket

```

| Parameter | Type | Description |
| --- | --- | --- |
| name | string | Name of the socket |

**Returns** `Socket`

#### name

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L21)

Name of the socket

```
tsname: string;

```

### Input

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L54)

The input port class

```
tsclass Input<S extends Socket>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | Socket |  |

**Extends** Port<S>

#### constructor

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L70)

```
tsconstructor(socket: S, label: string, multipleConnections: boolean): Input<S>

```

| Parameter | Type | Description |
| --- | --- | --- |
| socket | S | Socket instance |
| label | string | Label of the input port |
| multipleConnections | boolean | Whether the output port can have multiple connections. Default is false |

**Returns** `Input<S>`

#### control

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L58)

Control instance

```
tscontrol: Control;

```

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L33)

Port id, unique string generated by `getUID` function

```
tsid: string;

```

#### index

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L37)

Port index, used for sorting ports. Default is `0`

```
tsindex: number;

```

#### label

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L70)

Label of the input port

```
tslabel: string;

```

#### multipleConnections

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L70)

Whether the output port can have multiple connections. Default is `false`

```
tsmultipleConnections: boolean;

```

#### showControl

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L62)

Whether the control is visible. Can be managed dynamically by extensions. Default is `true`

```
tsshowControl: boolean;

```

#### socket

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L70)

Socket instance

```
tssocket: S;

```

#### addControl

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L78)

Add control to the input port

```
tsaddControl(control: Control): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| control | Control | Control instance |

**Returns** `void`

#### removeControl

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L86)

Remove control from the input port

```
tsremoveControl(): void

```

**Returns** `void`

### Control

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L111)

General control class

```
tsclass Control

```

**Extended by** InputControl

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L115)

Control id, unique string generated by `getUID` function

```
tsid: string;

```

#### index

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L119)

Control index, used for sorting controls. Default is `0`

```
tsindex: number;

```

### Output

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L95)

The output port class

```
tsclass Output<S extends Socket>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | Socket |  |

**Extends** Port<S>

#### constructor

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L102)

```
tsconstructor(socket: S, label: string, multipleConnections: boolean): Output<S>

```

| Parameter | Type | Description |
| --- | --- | --- |
| socket | S | Socket instance |
| label | string | Label of the output port |
| multipleConnections | boolean | Whether the output port can have multiple connections. Default is true |

**Returns** `Output<S>`

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L33)

Port id, unique string generated by `getUID` function

```
tsid: string;

```

#### index

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L37)

Port index, used for sorting ports. Default is `0`

```
tsindex: number;

```

#### label

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

Label of the port

```
tslabel: string;

```

#### multipleConnections

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

Whether the output port can have multiple connections

```
tsmultipleConnections: boolean;

```

#### socket

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

Socket instance

```
tssocket: S;

```

### InputControl

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L141)

The input control class

```
tsclass InputControl<T extends "text" | "number", N extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| T | `"text" | "number"` |
| N | unknown |  |

**Extends** Control

**Examples**

```
tsnew InputControl('text', { readonly: true, initial: 'hello' })

```

#### constructor

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L150)

```
tsconstructor(type: T, options: InputControlOptions<N>): InputControl<T, N>

```

| Parameter | Type | Description |
| --- | --- | --- |
| type | T | Type of the control: text or number |
| options | InputControlOptions<N> | Control options |

**Returns** `InputControl<T, N>`

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L115)

Control id, unique string generated by `getUID` function

```
tsid: string;

```

#### index

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L119)

Control index, used for sorting controls. Default is `0`

```
tsindex: number;

```

#### options

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L150)

Control options

```
tsoptions: InputControlOptions<N>;

```

#### type

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L150)

Type of the control: `text` or `number`

```
tstype: T;

```

#### setValue

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L162)

Set control value

```
tssetValue(value: N): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| value | N | Value to set |

**Returns** `void`

### Port

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L29)

General port class

```
tsclass Port<S extends Socket>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | Socket |  |

**Extended by** Input, Output

#### constructor

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

```
tsconstructor(socket: S, label: string, multipleConnections: boolean): Port<S>

```

| Parameter | Type | Description |
| --- | --- | --- |
| socket | S | Socket instance |
| label | string | Label of the port |
| multipleConnections | boolean | Whether the output port can have multiple connections |

**Returns** `Port<S>`

#### id

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L33)

Port id, unique string generated by `getUID` function

```
tsid: string;

```

#### index

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L37)

Port index, used for sorting ports. Default is `0`

```
tsindex: number;

```

#### label

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

Label of the port

```
tslabel: string;

```

#### multipleConnections

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

Whether the output port can have multiple connections

```
tsmultipleConnections: boolean;

```

#### socket

[presets/classic.ts](https://github.com/retejs/rete/blob/baa7397/src/presets/classic.ts#L45)

Socket instance

```
tssocket: S;

```

## Scope

[scope.ts](https://github.com/retejs/rete/blob/baa7397/src/scope.ts#L85)

Base class for all plugins and the core. Provides a signals mechanism to modify the data

```
tsclass Scope<Produces extends unknown, Parents extends unknown[]>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Produces | unknown |  |
| Parents | unknown[] |  |

**Extended by** NodeEditor

## Signal

[scope.ts](https://github.com/retejs/rete/blob/baa7397/src/scope.ts#L59)

A signal is a middleware chain that can be used to modify the data

```
tsclass Signal<T extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| T | unknown | The data type |

## BaseSchemes

[types.ts](https://github.com/retejs/rete/blob/baa7397/src/types.ts#L33)

The base schemes

```
tstype BaseSchemes = GetSchemes<NodeBase, ConnectionBase>;

```

## ConnectionBase

[types.ts](https://github.com/retejs/rete/blob/baa7397/src/types.ts#L20)

The base connection type

```
tstype ConnectionBase = { id: ConnectionId; source: NodeId; target: NodeId };

```

**Implemented by** Connection

## ConnectionId

[types.ts](https://github.com/retejs/rete/blob/baa7397/src/types.ts#L9)

Connection id type

```
tstype ConnectionId = string;

```

## GetSchemes

[types.ts](https://github.com/retejs/rete/blob/baa7397/src/types.ts#L27)

Get the schemes

```
tstype GetSchemes<
  NodeData extends NodeBase,
  ConnectionData extends ConnectionBase,
> = { Connection: ConnectionData; Node: NodeData };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| NodeData | NodeBase |  |
| ConnectionData | ConnectionBase |  |

**Examples**

```
tsGetSchemes<Node & { myProp: number }, Connection>

```

## NestedScope

[scope.ts](https://github.com/retejs/rete/blob/baa7397/src/scope.ts#L35)

Validate the Scope signals and replace the parameter type with an error message if they are not assignable

```
tstype NestedScope<
  S extends Scope<any, any[]>,
  Current extends any[],
> = CanAssignEach<Current, S["__scope"]["parents"]>[number] extends true
  ? S
  : "Parent signals do not satisfy the connected scope. Please use `.debug($ => $) for detailed assignment error";

```

| Parameter | Extends | Description |
| --- | --- | --- |
| S | Scope&lt;any, any[]&gt; |  |
| Current | any[] |  |

## NodeBase

[types.ts](https://github.com/retejs/rete/blob/baa7397/src/types.ts#L15)

The base node type

```
tstype NodeBase = { id: NodeId };

```

**Implemented by** Node

## NodeId

[types.ts](https://github.com/retejs/rete/blob/baa7397/src/types.ts#L4)

Node id type

```
tstype NodeId = string;

```

## Pipe

[scope.ts](https://github.com/retejs/rete/blob/baa7397/src/scope.ts#L17)

```
tstype Pipe<T extends unknown> = (
  data: T,
) => Promise<undefined | T> | undefined | T;

```

| Parameter | Extends | Description |
| --- | --- | --- |
| T | unknown | The data type |

## getUID

[utils.ts](https://github.com/retejs/rete/blob/baa7397/src/utils.ts#L10)

```
tsgetUID(): string

```

**Returns** `string`
A unique id

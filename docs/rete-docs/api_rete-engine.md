# API for rete-engine package

> Source: [https://retejs.org/docs/api/rete-engine](https://retejs.org/docs/api/rete-engine)

# API for rete-engine package

## DataflowEngine

[src/dataflow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow-engine.ts#L26)

DataflowEngine is a plugin that integrates Dataflow with NodeEditor making it easy to use.
Additionally, it provides a cache for the data of each node in order to avoid recurring calculations.

```
tsclass DataflowEngine<Schemes extends DataflowEngineScheme>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | DataflowEngineScheme |  |

**Extends** Scope<never, Root<Schemes>>

**Listens** `nodecreated` `noderemoved`

### fetch

[src/dataflow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow-engine.ts#L121)

Fetches output data of the node

```
tsfetch(nodeId: string): Promise<Record<string, any>>

```

**Throws** `Cancelled` when `reset` is called while fetching data

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id to fetch data from |

**Returns** `Promise<Record<string, any>>`

### fetchInputs

[src/dataflow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow-engine.ts#L112)

Fetches input data for the node by fetching data for all its predecessors recursively.

```
tsfetchInputs(nodeId: string): Promise<Record<string, any>>

```

**Throws** `Cancelled when `reset` is called while fetching data

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id to fetch input data for |

**Returns** `Promise<Record<string, any>>`

### reset

[src/dataflow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow-engine.ts#L90)

Resets the cache of the node and all its predecessors.

```
tsreset(nodeId: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id to reset. If not specified, all nodes will be reset. |

**Returns** `void`

## ControlFlowEngine

[src/control-flow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow-engine.ts#L22)

ControlFlowEngine is a plugin that integrates ControlFlow with NodeEditor making it easy to use

```
tsclass ControlFlowEngine<Schemes extends ControlFlowEngineScheme>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ControlFlowEngineScheme |  |

**Extends** Scope<never, Root<Schemes>>

**Listens** `nodecreated` `noderemoved`

### constructor

[src/control-flow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow-engine.ts#L29)

```
tsconstructor(configure: Configure<Schemes>): ControlFlowEngine<Schemes>

```

| Parameter | Type | Description |
| --- | --- | --- |
| configure | Configure<Schemes> | Allows to specify which inputs and outputs are part of the control flow |

**Returns** `ControlFlowEngine<Schemes>`

### execute

[src/control-flow-engine.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow-engine.ts#L73)

Trigger execution starting from the specified node.

```
tsexecute(nodeId: string, input: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id |
| input | string | Input key that will be considered as the initiator of the execution |

**Returns** `void`

## Dataflow

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L22)

Dataflow is a class that allows to process nodes in a graph using Dataflow approach.

```
tsclass Dataflow<Schemes extends ClassicScheme>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |

### constructor

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L28)

```
tsconstructor(editor: NodeEditor<Schemes>): Dataflow<Schemes>

```

| Parameter | Type | Description |
| --- | --- | --- |
| editor | NodeEditor<Schemes> | NodeEditor instance |

**Returns** `Dataflow<Schemes>`

### add

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L35)

Adds the node to the dataflow.

```
tsadd(node: T, setup: DataflowNodeSetup<T, any, any>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| node | T | Node instance |
| setup | DataflowNodeSetup<T, any, any> | Set of functions that define how to process the node |

**Returns** `void`

### fetch

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L92)

Fetches outputs of the node.
This method recursively calls `data` function of the predecessor nodes until receives all of the required inputs and calls `data` function of the specified node.

```
tsfetch(nodeId: string): Promise<Record<string, any>>

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id |

**Returns** `Promise<Record<string, any>>`
Object with outputs

### fetchInputs

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L58)

Fetches inputs of the node.
Unlike `fetch` method, this method doesn't call `data` function of the specified node (but does call `data` for predecessor nodes recursively).

```
tsfetchInputs(nodeId: string): Promise<Record<string, any>>

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id |

**Returns** `Promise<Record<string, any>>`
Object with inputs

### remove

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L48)

Removes the node from the dataflow.

```
tsremove(nodeId: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id |

**Returns** `void`

## ControlFlow

[src/control-flow.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow.ts#L21)

ControlFlow is a class that allows to execute nodes in a graph using Control flow approach.

```
tsclass ControlFlow<Schemes extends ClassicScheme>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ClassicScheme |  |

### constructor

[src/control-flow.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow.ts#L27)

```
tsconstructor(editor: NodeEditor<Schemes>): ControlFlow<Schemes>

```

| Parameter | Type | Description |
| --- | --- | --- |
| editor | NodeEditor<Schemes> | NodeEditor instance |

**Returns** `ControlFlow<Schemes>`

### add

[src/control-flow.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow.ts#L34)

Adds the node to the control flow.

```
tsadd(node: T, setup: ControlFlowNodeSetup<T, (keyof T["inputs"])[], (keyof T["outputs"])[]>): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| node | T | Node instance |
| setup | ControlFlowNodeSetup<T, (keyof T["inputs"])[], (keyof T["outputs"])[]> | Set of functions that define how to execute the node |

**Returns** `void`

### execute

[src/control-flow.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow.ts#L56)

Execute the node and its successors (in case `forward` is called for some output).

```
tsexecute(nodeId: string, input: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id |
| input | string | Input key that will be considered as the initiator of the execution |

**Returns** `void`

### remove

[src/control-flow.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow.ts#L47)

Removes the node from the control flow.

```
tsremove(nodeId: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| nodeId | string | Node id |

**Returns** `void`

## Cancelled

[src/utils/cancellable.ts](https://github.com/retejs/engine/blob/06c5665/src/utils/cancellable.ts#L4)

Cancelled exception. Thrown when `reset` is called while fetching data.

```
tsclass Cancelled

```

**Extends** Error

## ControlFlowNodeSetup

[src/control-flow.ts](https://github.com/retejs/engine/blob/06c5665/src/control-flow.ts#L8)

ControlFlowNodeSetup is a set of functions that define how to execute a node.

```
tstype ControlFlowNodeSetup<
  T extends ClassicScheme["Node"],
  I extends (keyof T["inputs"])[],
  O extends (keyof T["outputs"])[],
> = { inputs: Function; outputs: Function; execute: unknown };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| T | ClassicScheme[&quot;Node&quot;] |  |
| I | (keyof T[&quot;inputs&quot;])[] |  |
| O | (keyof T[&quot;outputs&quot;])[] |  |

## DataflowNodeSetup

[src/dataflow.ts](https://github.com/retejs/engine/blob/06c5665/src/dataflow.ts#L8)

DataflowNodeSetup is a set of functions that define how to process a node.

```
tstype DataflowNodeSetup<
  T extends ClassicScheme["Node"],
  I extends { [key in keyof T["inputs"]]: any },
  O extends { [key in keyof T["outputs"]]: any },
> = { inputs: Function; outputs: Function; data: unknown };

```

| Parameter | Extends | Description |
| --- | --- | --- |
| T | ClassicScheme[&quot;Node&quot;] |  |
| I | { [key in keyof T[&quot;inputs&quot;]]: any } |  |
| O | { [key in keyof T[&quot;outputs&quot;]]: any } |  |

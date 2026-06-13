# Data structures

> Source: [https://retejs.org/docs/guides/data-structures](https://retejs.org/docs/guides/data-structures)

# Data structures

Similar to a graph, this framework contains data as nodes and edges. One correction: in the editor's terminology, the graph edges are known as connections.

The `NodeEditor` instance stores this data in a normalized format, specifically as two distinct lists containing objects of the following type:

- `{ id: <string> }` for a nodes
- `{ id: <string>, source: <string>, target: <string> }` for a connections

NodeEditor
Initializing the editor involves using a basic scheme without any supplementary fields to start with:

```
tsimport { NodeEditor, BaseSchemes, getUID } from 'rete'

const editor = new NodeEditor<BaseSchemes>()

```

## Add nodes and connections

When dealing with graph data, you have the option to create arbitrary identifiers for nodes and connections, or utilize the existing ones.

```
tsconst a = { id: getUID() }
const b = { id: getUID() }
const connection = { id: getUID(), source: a.id, target: b.id }

await editor.addNode(a)
await editor.addNode(b)
await editor.addConnection(connection)

```

## Retrieve nodes and connections

We can now retrieve a list of newly added nodes and connections

```
tseditor.getNodes() // returns [a, b]
editor.getConnections() // returns [connection]

```

You have all the necessary methods to process the graph, such as retrieving a list of input connections or all incoming nodes, as we will discuss in the following sections.

## Incoming/outgoing connections

Incoming/outgoing connections
The following code shows how to retrieve a list of incoming and outgoing connections for `node`:

```
tsconst connections = editor.getConnections()

const incomingConnections = connections.filter(connection => connection.target === node.id)
const outgoingConnections = connections.filter(connection => connection.source === node.id)

```

## Incoming/outgoing nodes

Incoming/outgoing nodes
We can use variables from the previous section to retrieve incoming or outgoing nodes:

```
tsconst incomers = incomingConnections.map(connection => editor.getNode(connection.source))
const outgoers = outgoingConnections.map(connection => editor.getNode(connection.target))

```

In general, this is sufficient for simple cases, but if there are more than one connections between nodes, we will have to remove duplicates:

```
tsArray.from(new Set(incomers))
Array.from(new Set(outgoers))

```

## Advanced methods

The previously mentioned approaches are fairly flexible, but they require the implementation of more advanced methods on your own. Fortunately, the [rete-structures package](https://github.com/retejs/structures) offers such methods divided into 4 categories:

- **Mapping**: iterating through a list of nodes while transforming or filtering it
- **Sets**: techniques for working with graphs as sets, including union, difference and intersection.
- **Subgraph**: graphs that have parent-child relationships between nodes.
- **Traverse**: traversing nodes by their connections, such as retrieving only incoming nodes or all predecessors.

rete-structures
### Usage

Install the dependency

```
bashnpm i rete-structures

```

Use the following import statement and declaration

```
tsimport { structures } from 'rete-structures'

const graph = structures(editor)

graph.nodes()
graph.connections()

```

There are other `graph` methods that serve distinct purposes, demonstrated below with code and an interactive preview. The resulting nodes are prominent and the preview allows users to select nodes and observe changes.

The execution of any of the methods usually yields nodes that were not discarded, as well as their connections if both of their nodes are present.

### Traverse

#### Roots

Nodes without incoming connections are known as root nodes

```
tsstructures(graph).roots()

```

#### Leaves

Leaf nodes are those that have no outgoing connections

```
tsstructures(graph).leaves()

```

#### Incomers

Incoming nodes directly connected to the selected node

```
tsstructures(graph).incomers(selectedNodeId)

```

#### Outgoers

Outgoing nodes directly connected to the selected node

```
tsstructures(graph).outgoers(selectedNodeId)

```

#### Predecessors

Every incoming node, as well as its own incoming nodes and so on

```
tsstructures(graph).predecessors(selectedNodeId)

```

#### Successors

Every outgoing node, as well as its own outgoing nodes and so on

```
tsstructures(graph).successors(selectedNodeId)

```

### Mapping

#### Filter

Filtering can be applied to both nodes and connections

```
tsstructures(graph).filter(Boolean, ({ source, target }) => source === selectedNodeId || target === selectedNodeId)

```

### Sets

The following examples shows the cases where selected node serves as the second graph.

#### Union

The union of a graph and a node from this graph gives the same graph

```
tsstructures(editor).union({ nodes: [selectedNode], connections: [] })

```

#### Difference

By subtracting the node from the graph, you can obtain a new graph without that node

```
tsstructures(editor).difference({ nodes: [selectedNode], connections: [] })

```

#### Intersection

By intersecting the graph with the node from this graph, you'll get a new graph that includes only the selected node

```
tsstructures(editor).intersection({ nodes: [selectedNode], connections: [] })

```

### Subgraph

This category pertains to graphs that contain nodes with a parent-child relationship, specifically, nodes that have a `parent` field defined and are nested within other nodes.

#### Children

The list of direct descendant, namely children

```
tsstructures(editor).children((n) => n.id === selectedNodeId);

```

#### Parent

The list of parents

```
tsstructures(editor).parents((n) => n.id === selectedNodeId);

```

#### Descendants

The list of all descendants, including children, grandchildren and successive generations

```
tsstructures(editor).descendants((n) => n.id === selectedNodeId);

```

#### Ancestors

The list of all ancestors, from parents to great-grandparents and beyond

```
tsstructures(editor).ancestors((n) => n.id === selectedNodeId);

```

#### Orphans

Nodes without a specified `parent` property

```
tsstructures(editor).orphans();

```

#### Siblings

Nodes that have a common parent with the selected node, even if it has no parent

```
tsstructures(editor).siblings((n) => n.id === selectedNodeId)

```

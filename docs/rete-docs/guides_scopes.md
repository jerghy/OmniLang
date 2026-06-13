# Scopes

> Source: [https://retejs.org/docs/guides/scopes](https://retejs.org/docs/guides/scopes)

# Scopes

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

ScopesArrange nodes[Plugin](https://github.com/retejs/scopes-plugin)

The `rete-scopes-plugin` plugin provides features for building a node editor with nested nodes, also known as a subgraph. This plugin is categorized as advanced.

This approach involves using nodes as parent elements for other nodes. This enables the movement of a parent node to trigger the movement of all of its child nodes, and the movement of child nodes to adjust the size of their parent nodes. The nesting can extend to more than one level.

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-scopes-plugin

```

## Prepare nodes

The plugin requires node sizes to be explicitly specified:

```
tsclass Node extends ClassicPreset.Node {
  width = 190;
  height = 120;
  parent?: string
}
class Connection<N extends Node> extends ClassicPreset.Connection<N, N> {}

type Schemes = GetSchemes<Node, Connection<Node>>;

```

The purpose of this is to enable dynamic resizing of the node's sizes if it has nested nodes.

Additionally, it's suggested to specify an optional `parent` field to programmatically designate parent nodes.

## Plugin connection

```
tsimport { ScopesPlugin, Presets as ScopesPresets } from 'rete-scopes-plugin'

const scopes = new ScopesPlugin<Schemes>()

scopes.addPreset(ScopesPresets.classic.setup())

area.use(scopes)

```

This code relies on a classic preset that offers functionality for user-scope interactions. To select a node and assign it as a parent, the user may long-press the node and drag it over another node.

## Adding nodes

Let's add a couple of nodes, with one serving as a parent:

```
tsconst node1 = new Node('A');
const parent1 = new Node('Parent');

node1.parent = parent1.id; // specify node1 as nested into parent1

await editor.addNode(parent1); // make sure to add the parent node before adding its child
await editor.addNode(node1);

```

## Nodes order

Bu default, the plugin is set up to use its own node ordering. If you followed the Basic guide, you need to remove `AreaExtensions.simpleNodesOrder`

```
tsAreaExtensions.simpleNodesOrder(area);

```

The ordering of nested nodes is a bit different as parent nodes shouldn't overlap its child nodes. When bringing node to front or adding child nodes to it, the order of child nodes is adapted (by `reordered` event)

## Arrange nodes

The same Arrange nodes guide can be used for automated positioning of nodes in relation to one another, as the plugin used within also supports nested nodes.

## Dynamically changing relationships

After the nodes have been added to the editor, to change the bindings between nodes, in addition to changing `node.parent` you need to explicitly call the update method

```
ts// previously parent1.id was assigned
node1.parent = undefined;
await scopes.update(parent1.id)

```

## Change the parent node padding

By default, all nodes have the same padding. You can find these values in the API documentation. The padding can be customized for each node individually using the `padding` option.

```
tsconst scopes = new ScopesPlugin<Schemes>({
  padding: nodeId => ({ top: 40, left: 20, right: 20, bottom: 20 })
});

```

## Excluding nodes

The `exclude` option allows you to exclude specific nodes from being processed by the plugin. This can be useful when you want certain nodes not to affect the size of the parent node.

```
tsconst scopes = new ScopesPlugin<Schemes>({
  exclude: nodeId => excludedNodes.includes(nodeId)
});

```

## Changing parent node size

The generated sizes of parent nodes can be customized for specific nodes using the `size` option. This functionality is useful, for example, when child nodes have been removed, and you need to set a new size to display the layout correctly.

```
tsconst scopes = new ScopesPlugin<Schemes>({
  size: (nodeId, size) => ({ width: size.width, height: 100 })
});

```

Check out the complete result on the Scopes example page.

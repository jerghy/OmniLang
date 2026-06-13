# Context menu

> Source: [https://retejs.org/docs/guides/context-menu](https://retejs.org/docs/guides/context-menu)

# Context menu

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

Context menu[Plugin](https://github.com/retejs/context-menu-plugin)

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-context-menu-plugin

```

## Prepare nodes

To improve convenience and code reusability, it is recommended to create separate classes for nodes:

```
tsclass NodeA extends ClassicPreset.Node {
  constructor(socket: ClassicPreset.Socket) {
    super("NodeA");

    this.addControl("port", new ClassicPreset.InputControl("text", {}));
    this.addOutput("port", new ClassicPreset.Output(socket));
  }
}

/// class NodeB extends ...

type Node = NodeA | NodeB;
type Schemes = GetSchemes<Node, Connection<Node, Node>>;

```

## Plugin connection

For a simple solution, use the classic preset and specify a list of labeled items along with a function that returns the required node

```
tsimport { ContextMenuExtra, ContextMenuPlugin, Presets as ContextMenuPresets } from "rete-context-menu-plugin";

type AreaExtra = ReactArea2D<Schemes> | ContextMenuExtra;

const contextMenu = new ContextMenuPlugin<Schemes>({
  items: ContextMenuPresets.classic.setup([
    ["NodeA", () => new NodeA(socket)],
    ["NodeB", () => new NodeB(socket)]
  ])
});

area.use(contextMenu);

```

But this is not sufficient as the render plugin is responsible for visualization

## Rendering the context menu

Currently, the visualization of the context menu is possible using rendering plugins for **React.js**, **Vue.js**, **Angular**, **Svelte** and **Lit**.

```
tsimport { Presets } from "rete-react-plugin"; // or  rete-vue-plugin, rete-angular-plugin, rete-svelte-plugin, @retejs/lit-plugin

render.addPreset(Presets.contextMenu.setup());

```

For a comprehensive guide on how to connect a specific renderer plugin to your stack version, please follow the guide for
React.js, Vue.js, Angular, Svelte or Lit

Clicking on the free space opens up a menu that displays the available nodes for creation, or simply click on an existing node to delete it.

## Subitems

In order to specify node item as subitem, you can use the same definition using an array instead of a factory function:

```
tsconst contextMenu = new ContextMenuPlugin<Schemes>({
  items: ContextMenuPresets.classic.setup([
    ["Math", [
      ["Number", () => new NumberNode()],
    ]]
  ])
})

```

## Built-in options for nodes and connections

All nodes and connections have a `Delete` option in their context menu. This option allows you to delete a node, removing its connections first, or delete individual connections.

Another option that you won't see by default on nodes is `Clone`. It appears for nodes that have a `clone` method. For example:

```
tsclass NodeA extends ClassicPreset.Node {
  clone() {
    return new NodeA()
  }
}

```

## Custom preset

While the classic preset lets you briefly specify items for the main menu and node-specific menu, it might not offer enough flexibility. In such cases, you can define your own menu items:

```
tsconst contextMenu = new ContextMenuPlugin<Schemes>({
  items(context, plugin) {
    if (context === 'root') {
      return {
        searchBar: false,
        list: [
          { label: 'Custom', key: '1', handler: () => console.log('Custom') },
          {
            label: 'Collection', key: '1', handler: () => null,
            subitems: [
              { label: 'Subitem', key: '1', handler: () => console.log('Subitem') }
            ]
          }
        ]
      }
    }
    return {
      searchBar: false,
      list: []
    }
  }
})

```

where `context` is `'root'`, node instance or connection instance

## Open the menu programmatically

To manually open the context menu, you need to create a `PointerEvent` with the required coordinates and call the `area.emit()` method:

```
tsconst event = new PointerEvent('contextmenu', {
  clientX: x,
  clientY: y,
})

await area.emit({ type: 'contextmenu', data: { event, context } })

```

where

- `x`, `y` are numerical values (for example, the cursor coordinates, which you should extract separately),
- `context` can be `'root'` or an instance of a node, connection, or other elements in your editor.

Check out the complete result on the Context menu example page.

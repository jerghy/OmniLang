# Basic editor

> Source: [https://retejs.org/docs/guides/basic](https://retejs.org/docs/guides/basic)

# Basic editor

This guide features the `rete-react-plugin`. You can use it in any application, regardless of the framework you're using (**React.js**, **Vue.js**, **Angular**, **Svelte** etc.).

Follow this guide to use the corresponding render plugin in your **Angular**, **Svelte** or **Vue.js** application, with reference to the respective guides for Angular, Svelte, Vue.js etc.

BasicData structures[Core](https://github.com/retejs/rete)[Area plugin](https://github.com/retejs/area-plugin)[Connection plugin](https://github.com/retejs/connection-plugin)

In order to complete this guide, you can create an application on **Codesandbox** by choosing any client application template or set up the application locally.

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete rete-area-plugin rete-connection-plugin rete-react-plugin rete-render-utils styled-components react@19 react-dom@19

```

## Defining types and initializing the editor instance

```
tsimport { NodeEditor, GetSchemes, ClassicPreset } from "rete";

type Schemes = GetSchemes<
  ClassicPreset.Node,
  ClassicPreset.Connection<ClassicPreset.Node, ClassicPreset.Node>
>;

const editor = new NodeEditor<Schemes>();

```

where `Schemes` is a type that will assist you with type inference during plugin configuration. For more intricate examples, it may be necessary to extend the `ClassicPreset.Node` and `ClassicPreset.Connection` classes.

## Add an arbitrary node

Creating a node that contains one control and the output port. Keep in mind that `addNode` is an asynchronous method.

```
tsconst socket = new ClassicPreset.Socket("socket");

const nodeA = new ClassicPreset.Node("A");
nodeA.addControl("a", new ClassicPreset.InputControl("text", {}));
nodeA.addOutput("a", new ClassicPreset.Output(socket));
await editor.addNode(nodeA);

```

## Create an area to render with React.js

Place this code before calling `addNode`:

```
tsimport { createRoot } from "react-dom/client";
import { AreaPlugin } from "rete-area-plugin";
import { ReactPlugin, Presets, ReactArea2D } from "rete-react-plugin";

type AreaExtra = ReactArea2D<Schemes>;

const area = new AreaPlugin<Schemes, AreaExtra>(container);
const render = new ReactPlugin<Schemes, AreaExtra>({ createRoot });

render.addPreset(Presets.classic.setup());

editor.use(area);
area.use(render);

```

where

- `container` is the HTMLElement where the editor will be placed
- `AreaExtra` type enables the inclusion of custom viewable elements, as only `node` and `connection` are available by default

## Adding another node

```
tsconst nodeB = new ClassicPreset.Node("B");
nodeB.addControl("b", new ClassicPreset.InputControl("text", {}));
nodeB.addInput("b", new ClassicPreset.Input(socket));
await editor.addNode(nodeB);

```

Let's establish connection between these nodes

```
tsawait editor.addConnection(new ClassicPreset.Connection(nodeA, "a", nodeB, "b"));

```

## Node positioning

The screen will display two overlapping nodes, but we can adjust the position of the second node.

```
tsawait area.translate(nodeB.id, { x: 270, y: 0 });

```

## Interactive connections

This feature enables users to interact with the nodes.

```
tsimport { ConnectionPlugin, Presets as ConnectionPresets } from "rete-connection-plugin"

const connection = new ConnectionPlugin<Schemes, AreaExtra>();

connection.addPreset(ConnectionPresets.classic.setup())

area.use(connection);

```

## Fit viewport

Use the `zoomAt` extension to automatically adjust the viewport to fit all nodes.

```
tsimport { AreaExtensions } from "rete-area-plugin";

AreaExtensions.zoomAt(area, editor.getNodes());

```

By default, node dimensions are calculated using `clientWidth`/`clientHeight`. If this method is called right after appending nodes, it might not work correctly until the element is visible on the screen. Instead, you can specify `width`/`height` properties in the node class, as demonstrated in this step.

## Selectable nodes

Additionally, extensions offer various capabilities, like enabling the user to select nodes.

```
tsAreaExtensions.selectableNodes(area, AreaExtensions.selector(), {
  accumulating: AreaExtensions.accumulateOnCtrl()
});

```

For further details, check out the Selectable guide.

## Nodes order

If your application allows node selection, users may want to view selected nodes without any visual obstructions. To facilitate this, an additional extension is provided that automatically brings the selected node to the front.

```
tsAreaExtensions.simpleNodesOrder(area);

```

Check out the complete result on the Basic example page.

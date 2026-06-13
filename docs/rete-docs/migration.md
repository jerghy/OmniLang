# Migration

> Source: [https://retejs.org/docs/migration](https://retejs.org/docs/migration)

# Migration

[Rete.js v1](https://rete.js.org)

The current version of the framework contains numerous breaking changes compared to its predecessor.

Let's, start by exploring  the differences between v1 and v2, both from a developer's and user's point of view:

| Context | v1 | v2 | Reference |
| --- | --- | --- | --- |
| TypeScript | Partial support | TypeScript-first |  |
| Quick start | Codepen examples | DevKit, Codesandbox examples |  |
| Architecture | Event-based | Middleware-like signals |  |
| Tools | rete-cli | rete-cli, rete-kit, rete-qa |  |
| Testing | unit testing | unit + E2E testing |  |
| UI |  |  |  |
| Nodes order | fixed order | bring forward picked nodes |  |
| Selection | built-in for nodes only | advanced selection + custom elements |  |
| Controls | no built-in controls provided | built-in classic input control |  |
| Arrange nodes | limited | powered by elkjs |  |
| Code |  |  |  |
| Node creation | Component-based approach | up to you |  |
| Editor/Engine identifiers | mandatory, required for import/export | up to you |  |
| Node identifier | incremental decimal id | unique id |  |
| Import/export | Built-in, limited | up to you |  |
| Validation | Socket-based validation | up to you |  |
| Dataflow processing | limited (no recursion) | DataflowEngine with dynamic fetching |  |
| Control flow processing | simulated by Task plugin with limitations | ControlFlowEngine |  |
| Modules | rete-module-plugin | up to you |  |
| Connection plugin | responsible for both rendering and interaction | responsible for interaction only |  |

## Connecting plugins

Connect the plugin by importing it by default import.

The second parameter is used for passing the plugin's options/parameters:

```
ts// v1
import HistoryPlugin from 'rete-history-plugin';

editor.use(HistoryPlugin, { keyboard: true });

```

All plugins are implemented as classes and can be extended, providing flexible customization without modifying the core.

```
ts// v2
import { HistoryPlugin, HistoryExtensions, Presets } from 'rete-history-plugin'

const history = new HistoryPlugin<Schemes>()

history.addPreset(Presets.classic.setup())

HistoryExtensions.keyboard(history)

area.use(history)

```

## Creating nodes

In the v1, nodes are generated via components that were registered within the editor, which enabled the creation of numerous instances of nodes belonging to the same Component type.

```
ts// v1
class NumComponent extends Rete.Component {
  constructor(){
    super("Number");
  }

  builder(node) {
    node.addControl(new NumControl('num'))
    node.addOutput(new Rete.Output('num', "Number", numSocket))

    return node
  }
}

const numComponent = new NumComponent()
editor.register(numComponent);

const node = await numComponent.createNode({ num: 2 });

```

The current version doesn't include Component as an abstraction, but you can implement similar approach if needed.

```
ts// v2
const node = new ClassicPreset.Node('Number')

node.addControl('num', new NumControl('num'))
node.addOutput('num', new ClassicPreset.Output(numSocket, "Number"));

await editor.addNode(node)

```

## Saving data in a node

The data can be saved using method `putData`. It is expected that the data should be in a valid JSON format, as it may be used for import/export.

```
ts// v1
node.putData('myData', 'data')
control.putData('myData', 'data') // where control is part of node

```

There are no rigid import/export guidelines to follow in the current version, which means you have complete flexibility in how you store your data in nodes.

```
ts// v2
class MyNode extends ClassicPreset.Node {
  myData = 'data'
}

```

## Import/export

Because of the limitations mentioned earlier, the editor can be effortlessly exported and imported.

```
ts// v1
const data = editor.toJSON();
await editor.fromJSON(data);

```

The current version incorporates a revised approach that requires implementation, as demonstrated in Import/export.

## Selectable nodes

Selecting elements is a feature integrated within the editor

```
ts// v1
editor.selected.list

editor.selected.add(node, accumulate)

```

The downside to this implementation is its incapability to support anything other than node selection.

The selection of nodes (and other elements) looks like:

```
ts// v2
const selector = AreaExtensions.selector()
const accumulating = AreaExtensions.accumulateOnCtrl()

const nodeSelector = AreaExtensions.selectableNodes(area, selector, { accumulating });

editor.getNodes().filter(node => node.selected)
nodeSelector.select(add.id)

```

## Events listening

The typical way to listen to events that can be prevented

```
ts// v1
editor.on('nodecreate', node => {
 return node.canCreate
});

```

* - unchanged
** - moved to different package
*** - removed

### rete package events

- nodecreate *
- nodecreated *
- noderemove *
- noderemoved *
- connectioncreate *
- connectioncreated *
- connectionremove *
- connectionremoved *
- translatenode ***
- nodetranslate **
- nodetranslated **
- nodedraged ***
- nodedragged **
- selectnode ***
- multiselectnode ***
- nodeselect ***
- nodeselected ***
- rendernode ** (renamed to 'render')
- rendersocket ** (renamed to 'render')
- rendercontrol ** (renamed to 'render')
- renderconnection ** (renamed to 'render')
- updateconnection ***
- keydown ***
- keyup ***
- translate **
- translated **
- zoom **
- zoomed **
- click ** (renamed to 'nodepicked')
- mousemove *** (renamed to 'pointermove')
- contextmenu **
- import ***
- export ***
- process ***
- clear **

### rete-connection-plugin package events

- connectionpath **
- connectiondrop *
- connectionpick *
- resetconnection ***

The current version uses a specific kind of signal implementation that involves object-based signals. Additionally, pipes are used to either manipulate these objects or prevent signal propagation.

```
ts// v2
editor.addPipe(context => {
  if (context.type === 'nodecreate') return
  return context
})

```

### rete package events

- nodecreate
- nodecreated
- noderemove
- noderemoved
- connectioncreate
- connectioncreated
- connectionremove
- connectionremoved
- clear
- clearcancelled
- cleared

### rete-area-plugin package events

- nodepicked
- nodedragged
- nodetranslate
- nodetranslated
- contextmenu
- pointerdown
- pointermove
- pointerup
- noderesize
- noderesized
- render
- unmount
- reordered
- translate
- translated
- zoom
- zoomed
- resized

### rete-connection-plugin package events

- connectionpick
- connectiondrop

### rete-angular-plugin package events

- connectionpath

### rete-vue-plugin package events

- connectionpath

### rete-react-plugin package events

- connectionpath

## Validate connections

There is a built-in connection validation based on socket compatibility

```
ts// v1
const anyTypeSocket = new Rete.Socket('Any type');

numSocket.combineWith(anyTypeSocket);

```

This approach is simple but has some limitations.

Connection validation can be implemented independently, that provides more flexibility.

```
ts// v2
editor.addPipe(context => {
  if (context.type === 'connectioncreate') {
    if (canCreateConnection(context.data)) return false
  }
  return context
})

```

## Engine (dataflow)

The component with defined `worker` method should be registered

```
ts// v1
const engine = new Rete.Engine('demo@0.1.0');

engine.register(myComponent);

```

Define `worker` method of the component

```
ts// v1
worker(node, inputs, outputs){
  outputs['num'] = node.data.num;
}

```

Trigger the processing

```
ts// v1
await engine.process(data);

```

Create the `DataflowEngine` instance to connect to the editor. Unlike the first version, there is no need to pass `data` with nodes and connections.

```
ts// v2
import { DataflowEngine } from 'rete-engine'

const engine = new DataflowEngine<Schemes>()

editor.use(engine)

```

Node method example

```
ts// v2
data(inputs) {
  const { left, right } = inputs

  return { sum: left[0] + right[0] }
}

```

Start the processing

```
ts// v2
engine.fetch(node.id)

```

## Task plugin (control flow)

This approach is implemented using the `rete-task-plugin` and based on the `Rete.Engine`. Therefore, it has the aforementioned limitations

```
ts// v1
import TaskPlugin from 'rete-task-plugin';

editor.use(TaskPlugin);

```

Component's constructor has specified outputs that are intended for control flow or dataflow

```
ts// v1
this.task = {
    outputs: { exec: 'option', data: 'output' },
    init(task) {
        task.run('any data');
        task.reset();
    }
}

```

Define the `worker` method, which returns data and specifies closed output ports for control flow

```
ts// v1
worker(node, inputs, data) {
    this.closed = ['exec'];
    return { data }
}

```

The `rete-engine` package is used, which has a separate implementation of the engine for control flow

```
ts// v2
import { ControlFlowEngine } from 'rete-engine'

const engine = new ControlFlowEngine<Schemes>()

editor.use(engine)

```

By default, all ports are configured to pass control, but you can designate certain ones for this

```
ts// v2
const engine = new ControlFlowEngine<Schemes>(() => {
  return {
    inputs: () => ["exec"],
    outputs: () => ["exec"]
  };
});

```

The following serves as the node method:

```
ts// v2
execute(input: 'exec', forward: (output: 'exec') => void) {
  forward('exec')
}

```

Unlike the previous version, this approach is completely decoupled from the dataflow. Nevertheless, it can be used in conjunction with `DataflowEngine`.

```
ts// v2
async execute(input: 'exec', forward: (output: 'exec') => void) {
  const inputs = await dataflow.fetchInputs(this.id)

  forward('exec')
}

```

## Render plugins

As a demonstration, we have opted to use `rete-react-render-plugin`

```
ts// v1
import ReactRenderPlugin from 'rete-react-render-plugin';

editor.use(ReactRenderPlugin)

```

```
ts// v2
import { ReactPlugin } from 'rete-react-plugin'
import { createRoot } from "react-dom/client";

const reactPlugin = new ReactPlugin<Schemes, AreaExtra>({ createRoot })

area.use(reactPlugin)

```

### Custom nodes and controls

The following code is used to specify the components needed for specific nodes and controls

```
ts// v1
class AddComponent extends Rete.Component {
  constructor() {
    super("Add");
    this.data.component = MyNode;
  }
}

class MyControl extends Rete.Control {
  constructor(emitter, key, name) {
    super(key);
    this.render = 'react';
    this.component = MyReactControl;
    this.props = { emitter, name };
  }
}

```

Alternatively, component can be specified for all nodes

```
ts// v1
editor.use(ReactRenderPlugin, { component: MyNode });

```

In this version, the components to be visualized are defined in the classic preset that is connected

```
ts// v2
reactPlugin.addPreset(ReactPresets.classic.setup({ customize: {
  node(data) {
    return MyNode
  },
  control() {
    return MyReactControl
  }
}}))

```

This approach offers greater flexibility, enabling you to define additional conditions within the handlers

## Translate nodes

Retrieve the view of the node and execute its `translate` method

```
ts// v1
editor.view.nodes.get(node).translate(x, y)

```

The plugin instance contains `translate` method that only needs the node identifier.

```
ts// v2
await area.translate(node.id, { x, y })

```

## Arrange nodes

The plugin offers approach for positioning nodes, but its functionality is significantly restricted.

```
ts// v1
import AutoArrangePlugin from 'rete-auto-arrange-plugin';

editor.use(AutoArrangePlugin, {});

editor.trigger('arrange');

```

The plugin leverages the advanced functionality of the `elkjs` package.

```
ts// v2
import { AutoArrangePlugin, Presets as ArrangePresets } from "rete-auto-arrange-plugin";

const arrange = new AutoArrangePlugin<Schemes>();

arrange.addPreset(ArrangePresets.classic.setup());

area.use(arrange);

await arrange.layout()

```

## Fit viewport

The `zoomAt` method requires an editor instance that is responsible for visualization

```
ts// v1
import AreaPlugin from "rete-area-plugin";

AreaPlugin.zoomAt(editor);

```

For visualization purposes in this version, an instance of `AreaPlugin` is required.

```
ts// v2
import { AreaExtensions } from "rete-area-plugin";

AreaExtensions.zoomAt(area, editor.getNodes());

```

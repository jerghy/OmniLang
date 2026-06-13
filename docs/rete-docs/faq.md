# FAQ

> Source: [https://retejs.org/docs/faq](https://retejs.org/docs/faq)

# FAQ

## How to develop my own plugin?

Check out the instructions for Rete Kit

## Can the graph be processed on the server-side, such as Node.js?

Yes, follow the Dataflow or Control flow guides

## Can the graph be processed using Python or other server-side programming languages?

The engine only supports JS runtime. You can either bind JS into your environment or implement your own engine, similar to `rete-engine` (it has a straightforward implementation)

## Can the framework be used for commercial purposes?

In short, yes. Refer to the Licensing for details

## Can I use the framework if I don't know TypeScript?

It can be used in JS code, but not recommended because of poor DX

## Is there a way to save a graph as a JSON file?

Yes, follow the Import/export guide

## How to disable scroll prevention?

Replace zoom handler with `null` or use a custom one

```
tsconst area = new AreaPlugin(container)

area.area.setZoomHandler(null)

```

## How to enable/disable zoom dynamically?

Replace the zoom handler with `null` on some event and restore it on opposite  (e.g. Ctrl press/release)

```
tsimport { Zoom } from 'rete-area-plugin'

// call on init
area.area.setZoomHandler(null)

// call on Ctrl press
area.area.setZoomHandler(new Zoom(0.1))

// call on Ctrl release
area.area.setZoomHandler(null)

```

## How to pan the area using the middle mouse button?

Replace the drag handler for a specific area by calling `setDragHandler`

```
ts  import { Drag } from 'rete-area-plugin';

  area.area.setDragHandler(new Drag({
    down: e => {
      if (e.pointerType === 'mouse' && e.button !== 1) return false

      e.preventDefault()
      return true
    },
    move: () => true
  }))

```

## How can I retrieve the position of a node?

The position is stored within a `NodeView` instance.

```
ts  const view = area.nodeViews.get(nodeId)

  if (view) {
    view.position // { x, y }
  }

```

Keep in mind that the `NodeView` instance may not exist, for example if it hasn't yet been added. In such cases, it's preferable to handle this gracefully (throw an exception only when necessary)

## How can I translate a node/change the position of a node?

```
ts  area.translate(nodeId, { x: 0, y: 0 })

```

## How can I automatically arrange the position of nodes?

Follow the Arrange nodes guide

## How to force an update on nodes or controls?

Call the following methods for the corresponding nodes or controls after making changes to the state

```
tsconst area = new AreaPlugin(container)

area.update('node', node.id)
area.update("control", control.id);

```

## How to render various nodes or controls using Angular, React, and Vue in one editor?

Check out the Combining render plugins in the Integration article

## How can I add my own elements to the editor area?

Use the methods of `content` property of the area plugin

```
tsconst area = new AreaPlugin(container)

area.content.add(element)

area.content.remove(element)

```

It's also possible to make this element draggable

```
tsconst dragHandler = new Drag()

dragHandler.initialize(element, { /* getters */ }, { /* events */ })

```

## How to customize nodes?

Check out the relevant customization guide for React.js, Vue.js, Angular, Svelte or Lit.

## How to collapse the node (to minimize a node's size by hiding its controls)?

Define how elements should be hidden when the node is collapsed by creating a custom node component.

Check out the relevant customization guide for React.js, Vue.js, Angular, Svelte or Lit.

## What are the steps to implement render plugin for other framework?

- Setup a plugin
- Use the source code of the following packages as a reference: [React.js](https://github.com/retejs/react-plugin), [Vue.js](https://github.com/retejs/vue-plugin), [Angular](https://github.com/retejs/angular-plugin), [Svelte](https://github.com/retejs/svelte-plugin) or [Lit](https://github.com/retejs/lit-plugin)

## Is there way to prevent nodes from being moved while interacting with controls?

You need to stop propagation of the `pointerdown` event.

Check the relevant render plugin guide for controls: React.js, Vue.js, Angular, Svelte, Lit

## Why doesn't the control capture click/pointer events?

By default, the area captures these events, so you need to stop the propagation of `pointerdown` event to prevent this

Check the relevant render plugin guide for controls: React.js, Vue.js, Angular, Svelte, Lit

## Is there a way to make the text within a node selectable?

By default, a node is configured with the CSS property `user-select: none` to prevent conflict between text selection and node dragging.

Therefore, to enable text selection within a custom node component, you need to specify `user-select: all` property for the desired element within the node. Additionally, ensure you call `e.stopPropagation()` on the `pointerdown`` event to prevent text selection interruptions while dragging.

## How to detect click on the node?

Whenever a user clicks on the node, the `nodepicked` event is fired:

```
tsarea.addPipe(context => {
  if (context.type === 'nodepicked') {
    const node = editor.getNode(context.data.id)

  }
  return context
})

```

In case you need to track not only click, but also node selections, you have the option to extend the selector to observe selected elements (not just nodes) within the editor

## How to add inputs, outputs or controls dynamically?

You can add them as usual using the `addInput`/`addOutput`/`AddControl` methods, and then force a node update

```
tsconst area = new AreaPlugin(container)

area.update('node', node.id)

```

## How to add a control to the output?

The process of adding such elements requires the creation of a custom node.

Check out the relevant customization guide for React.js, Vue.js, Angular, Svelte or Lit.

## How to create an undirected graph with nodes that don't have input/output sockets?

You can use the classic preset with custom nodes and a unified socket for both input and output port.

Check out the Undirected example

## How to make an editor oriented vertically?

Check out the Vertical flow example

## How to change the order of inputs/controls/outputs?

The classic rendering preset offers the flexibility to specify an optional `index` field for inputs, outputs or controls. This feature enables you to change the order of these elements within their lists.

```
tsconst input = new ClassicPreset.Input(socket)
const output = new ClassicPreset.Output(socket)
const control = new ClassicPreset.InputControl('text')

input.index = 0;
output.index = 0;
control.index = 0;

```

## How to change the alignment of inputs/outputs?

The process of changing the node layout requires the creation of a custom node.

Check out the relevant customization guide for React.js, Vue.js, Angular, Svelte or Lit.

## Is it possible to render UI using vanilla JS exclusively, without resorting to frameworks?

In short, it is possible, but there is no plugin available for this approach since it doesn't offer significant advantages in comparison to the development costs

## What is the best way to update all `rete` related dependencies?

If you don't want to update all of your dependencies at once with `npm update`, you can selectively update those that begin with `rete` using a regular expression

```
bashnpx npm-check-updates /^rete/ --target @latest -u

```

## How to display a loop connection?

All available rendering plugins can display a loop connection if it has specified `isLoop` property

```
tsclass Connection extends ClassicPreset.Connection {
  isLoop = false
}

const connection = new Connection(source, output, target, input)

connection.isLoop = true

```

## What is the minimal TypeScript version required?

The minimum required TypeScript version is 4.7.

Otherwise, you might encounter the error `Type instantiation is excessively deep and possibly infinite. ts(2589)` when using the `use` method. If you can't use a later version for some reason, the only solution is to use `@ts-ignore`.

For example, when creating an Angular 12 application with Rete Kit, a version 4.7 higher than the officially supported one is installed.

## How can I change the editor's zoom level programmatically?

Using the `area.zoom` method enables you to specify the desired zoom level and the offset points for aligning the zoom

```
tsawait area.area.zoom(0.8, 0, 0);

```

In the given example, the zoom will be decreased with respect to the top left boundary. If you want to modify the zoom relative to the center of the viewport, refer to the following code

```
tsconst delta = 0.2;
const { k } = area.area.transform;
const box = area.container.getBoundingClientRect();
const x = box.width / 2 / k;
const y = box.height / 2 / k;

area.area.zoom(k * (1 - delta), x * delta, y * delta);

```

## How can I modify the position of the area?

By utilizing the `area.translate` method, you can change the coordinates as follows

```
tsawait area.area.translate(100, 20)

```

Change in position relative to current coordinates

```
tsconst { x, y } = area.area.transform
await area.area.translate(x + 100, y + 20)

```

Change the editor's position considering the zoom factor

```
tsconst { k } = area.area.transform

await area.area.translate(100 * k, 20 * k)

```

## What if my app uses different stack and isn't based on React.js, Vue.js, or Angular?

In case you're using a framework other than React.js, Vue.js, Angular, Svelte or Lit (for which Rete.js provides a rendering plugin), you have the option to utilize the React.js plugin to render nodes and other editor elements.

For a quick start, you can create a React.js application using Rete Kit, copy the code of the editor from `src/rete/default.tsx`, install the relevant dependencies in your project and call `createEditor`, providing the HTMLElement container created by your application.

## How can I retrieve the coordinates of the viewport center?

First of all, you need to obtain the viewport center in screen coordinates using `getBoundingClientRect`. Afterward, you should transform them into editor coordinates by applying the zoom factor `k` and offset it relative to area's position.

```
tsconst area = new AreaPlugin<Schemes, AreaExtra>(container)

const { x, y, k } = area.area.transform
const box = area.container.getBoundingClientRect()
const halfWidth = box.width / 2 / k
const halfHeight = box.height / 2 / k

return { x: halfWidth - x / k, y: halfHeight - y / k }

```

## What does the warning 'Found more than one element for socket with same key and side' mean?

The warning "Found more than one element for socket with same key and side" means that there are duplicate sockets in the editor that were not properly removed after an update, or due to an asynchronous approach when unmounting components by different UI frameworks, the lifecycle of old and new ones overlaps.

In the first case, when the number of these warnings constantly accumulates, you most likely have a memory leak problem because some custom node was not correctly unmounted.

In the second case, the warning can be avoided by adding a delay between removing the scheme/node and mounting a new one with the same identifiers. In the worst case, this warning should not indicate memory leak issues, so you can ignore it.

## How to track the select or deselect event of a node?

The framework does not have a reserved event for this. Instead, you can [/docs/guides/selectable#extend-selector](extend the selector) by triggering events in the relevant methods.

## How to handle double-click events on nodes?

Create a custom node component with a `dblclick` event handler.

Check out the customization guide for your renderer: React.js, Vue.js, Angular, Svelte or Lit.

**Important:** If your custom node contains nested interactive elements (buttons, inputs, etc.), make sure to prevent event propagation to avoid conflicts with the node's double-click handler.

## How to prevent/disable zoom on dblclick?

```
ts  area.addPipe(context => {
    if (context.type ===  'zoom' && context.data.source === 'dblclick') return
    return context
  })

```

# Svelte

> Source: [https://retejs.org/docs/guides/renderers/svelte](https://retejs.org/docs/guides/renderers/svelte)

# Svelte

This guide is an extension of the Basic guide and provides instructions for using the `rete-svelte-plugin` instead of `rete-react-plugin`

BasicCustomizationControls[Plugin](https://github.com/retejs/svelte-plugin)[Svelte](https://svelte.dev/)Context menuMinimapReroute

This plugin offers a classic preset that comes with visual components for nodes, connections, sockets, and input controls.

Supports latest versions of Svelte: 5, 4 and 3

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-svelte-plugin rete-render-utils sass

```

## Plugin connection

```
tsimport { AreaPlugin } from "rete-area-plugin";
import { SveltePlugin, Presets, SvelteArea2D } from "rete-svelte-plugin/5"; // or "rete-svelte-plugin" for older versions

type AreaExtra = SvelteArea2D<Schemes>;

// ....

const render = new SveltePlugin<Schemes, AreaExtra>();

render.addPreset(Presets.classic.setup());

area.use(render);

```

Please note that this plugin is intended for client-side use only. Therefore, modules that use it within your `*.svelte` modules need to be dynamically imported.

```
tsonMount(async () => {
  const { createEditor } = await import("./editor");
})

```

Check out the Svelte page for an example usage of this render plugin.

## Controls

This plugin provides built-in controls that are displayed based on the following objects:

- `ClassicPreset.InputControl` as `<input type="number" />` or `<input type="text" />`

Simply add the control to the node

```
tsnode.addControl('my-control', new ClassicPreset.InputControl("number", {
  initial: 0,
  readonly: false,
  change(value) { }
}))

```

If you want to add different types of controls, you can return the necessary component in the `control` handler of `customize` property.

```
tsximport MyButton from './MyButton.svelte'

render.addPreset(Presets.classic.setup({
  customize: {
    control(context) {
      if (context.payload.isButton) {
        return MyButton
      }
      if (context.payload instanceof ClassicPreset.InputControl) { // don't forget to explicitly specify the built-in Presets.classic.Control
        return Presets.classic.Control;
      }
    }
  }
}));

node.addControl('my-button', { isButton: true, label: 'Click', onClick() {} })

```

**MyButton.svelte**

```
svelte<button
  on:pointerdown|stopPropagation={() => null}
  on:click="data.onClick"
>
  {data.label}
</button>

```

This is a simplified version suitable for introductory purposes. For projects, it is recommended to follow the approach demonstrated in the example

Make sure to specify `on:pointerdown|stopPropagation` to prevent the area from intercepting events such as `click`.

## Customization

In a similar manner to the approach outlined above, you can replace node, connection, or socket components.

### Customization of all nodes

If you want to completely change the node structure, you can implement your own component similar to [Node.svelte](https://github.com/retejs/svelte-plugin/blob/main/src/presets/classic/components/Node.svelte) from the classic preset

```
tsimport CustomNode from './CustomNode.svelte'

render.addPreset(Presets.classic.setup({
  customize: {
    node() {
      return CustomNode
    }
  }
}))

```

The implementation of `CustomNode` is available in the **CustomNode.svelte** file of the Customization for Svelte example.

### Specific nodes

You can add an extra condition to apply this component only to specific nodes.

```
tsrender.addPreset(Presets.classic.setup({
  customize: {
    node(context) {
      if (context.payload.label === "Custom") {
        return CustomNode;
      }
      return Presets.classic.Node;
    }
  }
}))

await editor.addNode(new ClassicPreset.Node('White'))

```

### Connection customization

Use **Connection.svelte** as a starting point from the [presets/classic/components](https://github.com/retejs/svelte-plugin/blob/main/src/presets/classic/components) directory of the plugin's source code.

```
tsimport CustomConnection from './CustomConnection.svelte'

render.addPreset(Presets.classic.setup({
  customize: {
    connection() {
      return CustomConnection
    }
  }
}))

```

### Socket customization

Use **Socket.svelte** as a starting point from the [presets/classic/components](https://github.com/retejs/svelte-plugin/blob/main/src/presets/classic/components) directory of the plugin's source code.

```
tsimport CustomSocket from './CustomSocket.svelte'

render.addPreset(Presets.classic.setup({
  customize: {
    socket() {
      return CustomSocket
    }
  }
}))

```

## Customize context menu

In order to customize the context menu for this rendering plugin, one can override styles using selectors (and it's important to consider the specificity of selectors in CSS)

```
scss.rete-context-menu {
  width: 320px !important;
  .block:first-child input {
    background: grey;
  }
  .block.item {
    background: grey;
  }
}

```

## Other presets

- context menu
- minimap
- reroute

Check out the complete result on the Customization for Svelte example page.

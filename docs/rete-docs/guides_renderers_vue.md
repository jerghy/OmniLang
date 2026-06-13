# Vue.js

> Source: [https://retejs.org/docs/guides/renderers/vue](https://retejs.org/docs/guides/renderers/vue)

# Vue.js

This guide is an extension of the Basic guide and provides instructions for using the `rete-vue-plugin` instead of `rete-react-plugin`

BasicCustomizationControls[Plugin](https://github.com/retejs/vue-plugin)[Vue.js](https://vuejs.org/)Context menuMinimapReroute

This plugin offers a classic preset that comes with visual components for nodes, connections, sockets, and input controls.

Supports both versions of Vue.js: 2 and 3

You can use this plugin in any application, irrespective of the application stack (React.js, Vue.js, Angular). However, using SFC requires a bundler with a corresponding template compiler installed, which is a separate topic for discussion.

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-vue-plugin rete-render-utils

```

## Plugin connection

```
tsimport { AreaPlugin } from "rete-area-plugin";
import { VuePlugin, Presets, VueArea2D } from "rete-vue-plugin";

type AreaExtra = VueArea2D<Schemes>;

// ....

const render = new VuePlugin<Schemes, AreaExtra>();

render.addPreset(Presets.classic.setup());

area.use(render);

```

Check out the Vue page(/examples/basic/vue) page for an example usage of this render plugin.

## Using Vue.js 2

To use the plugin with Vue 2, add `/vue2` to the import statement.

```
tsimport { VuePlugin, Presets, VueArea2D } from "rete-vue-plugin/vue2";

```

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
tsximport MyButton from './MyButton.vue'

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

**MyButton.vue**

```
vue<template>
<button
  @pointerdown.stop=""
  @click="data.onClick"
>{{data.label}}</button>
</template>

```

This is a simplified version suitable for introductory purposes. For projects, it is recommended to follow the approach demonstrated in the example

Make sure to specify `@pointerdown.stop` to prevent the area from intercepting events such as `click`.

## Customization

In a similar manner to the approach outlined above, you can replace node, connection, or socket components.

### Customization of all nodes

If you want to completely change the node structure, you can implement your own component similar to [Node.vue](https://github.com/retejs/vue-plugin/blob/next/src/presets/classic/components/Node.vue) from the classic preset

```
tsimport CustomNode from './CustomNode.vue'

render.addPreset(Presets.classic.setup({
  customize: {
    node() {
      return CustomNode
    }
  }
}))

```

The implementation of `CustomNode` is available in the **CustomNode.vue** file of the Customization for Vue.js example.

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

Use **Connection.vue** as a starting point from the [presets/classic/components](https://github.com/retejs/vue-plugin/blob/next/src/presets/classic/components) directory of the plugin's source code.

```
tsimport CustomConnection from './CustomConnection.vue'

render.addPreset(Presets.classic.setup({
  customize: {
    connection() {
      return CustomConnection
    }
  }
}))

```

### Socket customization

Use **Socket.vue** as a starting point from the [presets/classic/components](https://github.com/retejs/vue-plugin/blob/next/src/presets/classic/components) directory of the plugin's source code.

```
tsimport CustomSocket from './CustomSocket.vue'

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
scss[rete-context-menu] {
  width: 320px !important;
  .block:first-child input {
    background: grey;
  }
  .block.item {
    background: grey;
  }
}

```

## Use Vue.js plugins

Since `rete-vue-plugin` creates independent Vue.js instance for nodes, sockets, controls, etc., it doesn't inherit plugins from your project's main Vue instance. To bridge this gap, the plugin offers a solution: injecting a custom Vue application instance. This capability ensures that any Vue plugins or global components you wish to employ within your Rete-specific Vue components are accessible, enabling seamless sharing between your Vue.js application and Rete.js editor.

### Vue.js 3

The following example demonstrates how to configure custom Vue.js 3 instance:

```
tsimport { Presets, VuePlugin } from "rete-vue-plugin";
import { createApp } from "vue";

const render = new VuePlugin<Schemes, AreaExtra>({
  setup(context) {
    const app = createApp(context);

    app.use(yourPlugin);

    return app;
  },
});

```

where `yourPlugin` is an instance of any plugin (like [Vuetify](https://vuetifyjs.com/en/getting-started) or [Vue I18N](https://vue-i18n.intlify.dev/))

### Vue.js 2

Since the initialization for Vue.js 2 is slightly different, let's take a look at the following example:

```
tsimport { Presets, VuePlugin } from "rete-vue-plugin";
import Vue from "vue";

const render = new VuePlugin<Schemes, AreaExtra>({
  setup(context) {
    const app = new Vue({ ...context, yourPlugin });

    return app;
  },
});

```

where `yourPlugin` is an instance of any plugin (like [Vuetify](https://vuetifyjs.com/en/getting-started) or [Vue I18N](https://kazupon.github.io/vue-i18n/))

## Other presets

- context menu
- minimap
- reroute

Check out the complete result on the Customization for Vue.js example page.

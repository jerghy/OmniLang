# Lit

> Source: [https://retejs.org/docs/guides/renderers/lit](https://retejs.org/docs/guides/renderers/lit)

# Lit

This guide is an extension of the Basic guide and provides instructions for using the `@retejs/lit-plugin` instead of `rete-react-plugin`

BasicCustomizationControls[Plugin](https://github.com/retejs/lit-plugin)[Lit](https://lit.dev/)Context menuMinimapReroute

This plugin offers a classic preset that comes with visual components for nodes, connections, sockets, and input controls.

Supports Lit version 3

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i @retejs/lit-plugin rete-render-utils lit

```

## Plugin connection

```
tsimport { AreaPlugin } from "rete-area-plugin";
import { LitPlugin, Presets, LitArea2D } from "@retejs/lit-plugin";

type AreaExtra = LitArea2D<Schemes>;

// ....

const render = new LitPlugin<Schemes, AreaExtra>();

render.addPreset(Presets.classic.setup());

area.use(render);

```

Check out the Lit page for an example usage of this render plugin.

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
tsimport { MyButtonElement } from './MyButton'

customElements.define("my-button", MyButtonElement);

render.addPreset(Presets.classic.setup({
  customize: {
    control(context) {
      if (context.payload.isButton) {
        const { payload } = context;

        return () => html`<my-button .data=${payload}></my-button>`;
      }
      if (context.payload instanceof ClassicPreset.InputControl) { // don't forget to explicitly specify the built-in <rete-control>
        return () => html`<rete-control .data=${payload}></rete-control>`;
      }
    }
  }
}));

node.addControl('my-button', { isButton: true, label: 'Click', onClick() {} })

```

**MyButton.ts**

```
tsexport class CustomButton extends LitElement {
  static get properties() {
    return {
      data: { type: Object },
    };
  }

  declare data: object;

  render() {
    return html`
      <button
        @pointerdown=${(e: MouseEvent) => e.stopPropagation()}
        @doubleclick=${(e: MouseEvent) => e.stopPropagation()}
        @click=${this.data.onClick}
      >${this.data.label}</button>
    `;
  }
}

```

This is a simplified version suitable for introductory purposes. For projects, it is recommended to follow the approach demonstrated in the example

Make sure to specify `@pointerdown` and `@doubleclick` to prevent the area from intercepting events such as `@click`.

## Customization

In a similar manner to the approach outlined above, you can replace node, connection, or socket components.

### Customization of all nodes

If you want to completely change the node structure, you can implement your own component similar to [node.ts](https://github.com/retejs/lit-plugin/blob/main/src/presets/classic/components/node.ts) from the classic preset

```
tsimport { CustomNodeElement } from './CustomNode'

customElements.define("custom-node", CustomNodeElement)

render.addPreset(Presets.classic.setup({
  customize: {
    node(context) {
      return ({ emit }) => html`<custom-node .data=${context.payload} .emit=${emit}></custom-node>`;
    }
  }
}))

```

The implementation of `CustomNodeElement` is available in the **CustomNode.ts** file of the Customization for Lit example.

### Specific nodes

You can add an extra condition to apply this component only to specific nodes.

```
tsrender.addPreset(Presets.classic.setup({
  customize: {
    node(context) {
      if (context.payload.label === "Custom") {
        return ({ emit }) => html`<custom-node .data=${context.payload} .emit=${emit}></custom-node>`;
      }
      return ({ emit }) => html`<rete-node .data=${context.payload} .emit=${emit}></rete-node>`;
    }
  }
}))

await editor.addNode(new ClassicPreset.Node('White'))

```

### Connection customization

Use **connection.ts** as a starting point from the [presets/classic/components](https://github.com/retejs/lit-plugin/blob/main/src/presets/classic/components/connection.ts) directory of the plugin's source code.

```
tsimport { CustomConnectionElement } from './CustomConnection'

customElements.define("custom-connection", CustomConnectionElement);

render.addPreset(Presets.classic.setup({
  customize: {
    connection() {
      return ({ path }) => html`<custom-connection .path=${path}></custom-connection>`;
    }
  }
}))

```

### Socket customization

Use **socket.ts** as a starting point from the [presets/classic/components](https://github.com/retejs/lit-plugin/blob/main/src/presets/classic/components/socket.ts) directory of the plugin's source code.

```
tsimport { CustomSocketElement } from './CustomSocket'

customElements.define("custom-socket", CustomSocketElement);

render.addPreset(Presets.classic.setup({
  customize: {
    socket(context) {
      return () => html`<custom-socket .data=${context.payload}></custom-socket>`;
    }
  }
}))

```

## Other presets

- context menu
- minimap
- reroute

Check out the complete result on the Customization for Lit example page.

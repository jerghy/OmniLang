# React.js

> Source: [https://retejs.org/docs/guides/renderers/react](https://retejs.org/docs/guides/renderers/react)

# React.js

This guide is an extension of the Basic guide and provides in-depth instructions for using the `rete-react-plugin`

BasicControlsCustomization[Plugin](https://github.com/retejs/react-plugin)[React.js](https://react.dev/)Context menuMinimapReroute

This plugin uses a classic preset that includes visual components for nodes, connections, sockets, and input controls. It leverages `styled-components` to design these components.

This plugin can be used in any application, regardless of your stack (**React.js**, **Vue.js**, **Angular**, etc.).

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-react-plugin rete-render-utils styled-components

```

If you're using this plugin in an application that doesn't utilize **React.js**, make sure to install the required **React.js** dependencies as well.

```
bashnpm i react@19 react-dom@19

```

## Plugin connection

```
tsimport { createRoot } from "react-dom/client";
import { AreaPlugin } from "rete-area-plugin";
import { ReactPlugin, Presets, ReactArea2D } from "rete-react-plugin";

type AreaExtra = ReactArea2D<Schemes>;

// ....

const render = new ReactPlugin<Schemes, AreaExtra>({ createRoot });

render.addPreset(Presets.classic.setup());

area.use(render);

```

**React 19 Requirement**: When using React 19, passing `createRoot` as a parameter is **mandatory** when initializing ReactPlugin. This is required due to changes in React 19's rendering system.

Check out the Basic page for an example of how to use this rendering plugin.

## Using React.js 16-17

In case you're using React.js version 16 or 17, the `createRoot` method is optional

```
tsconst render = new ReactPlugin<Schemes, AreaExtra>();

```

## "useRete" hook

When working with React app, `useRete` hook eliminates the need for boilerplate code that binds an editor to HTML element. This becomes particularly crucial for dynamic app updates where the old instance of the editor must be removed and replaced with a new one.

```
tsximport { useRete } from 'rete-react-plugin';

function App() {
  const [ref, editor] = useRete(createEditor)

  return <div ref={ref} className="rete"></div>
}

```

where `createEditor` should return object with `destroy` method (usually it has `area.destroy()` call)

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

If you want to add different types of controls, you can return the necessary functional component in the `control` handler of `customize` property.

```
tsxrender.addPreset(Presets.classic.setup({
  customize: {
    control(context) {
      if (context.payload.isButton) {
        return (props: { data: { isButton: true, label: string, onClick: () => void }}) => (
          <button
            onPointerDown={(e) => e.stopPropagation()}
            onClick={props.data.onClick}
          >
            {props.data.label}
          </button>
        )
      }
      if (context.payload instanceof ClassicPreset.InputControl) { // don't forget to explicitly specify the built-in Presets.classic.Control
        return Presets.classic.Control;
      }
    }
  }
}));

node.addControl('my-button', { isButton: true, label: 'Click', onClick() {} })

```

This is a simplified version suitable for introductory purposes. For projects, it is recommended to follow the approach demonstrated in the example

Make sure to call `stopPropagation` in `onPointerDown` to prevent the area from intercepting events such as `click`. If you are encountering this issue in React 16 or your interactive elements are added to a custom node instead of a control, try the following solution:

```
tsximport { Drag } from "rete-react-plugin";

<Drag.NoDrag>
  <button>
    {props.data.label}
  </button>
</Drag.NoDrag>

```

Or use a hook to avoid extra nesting

```
tsxconst ref = React.useRef(null)

Drag.useNoDrag(ref)

<button ref={ref}>
  {props.data.label}
</button>

```

## Customization

In a similar manner to the approach outlined above, you can replace node, connection, or socket components.

### Node styles

The easiest approach is to extend the current component and use `styled-components` to add extra styles.

```
tsximport { Presets } from "rete-react-plugin";
import { css } from "styled-components";

const myStyles = css<{ selected?: boolean }>`
  background: white;
  ${(props) => props.selected && css`
    border-color: red;
  `}
`;

function StyledNode(props: { data: Schemes['Node'] }) {
  return <Presets.classic.Node styles={() => myStyles} {...props} />;
}

render.addPreset(Presets.classic.setup({
  customize: {
    node() {
      return StyledNode
    }
  }
}))

```

Implementing this will result in all your nodes using `myStyles`.

### Specific nodes

You can add an extra condition to apply these styles only to specific nodes.

```
tsrender.addPreset(Presets.classic.setup({
  customize: {
    node(context) {
      if (context.payload.label === "White") {
        return StyledNode;
      }
      return Presets.classic.Node;
    }
  }
}))

await editor.addNode(new ClassicPreset.Node('White'))

```

### Full node customization

If you want to completely change the node structure, you can implement your own component similar to [Node.tsx](https://github.com/retejs/react-plugin/blob/next/src/presets/classic/components/Node.tsx) from the classic preset.

```
tsimport { CustomNode } from './CustomNode'

render.addPreset(Presets.classic.setup({
  customize: {
    node() {
      return CustomNode
    }
  }
}))

```

The implementation of `CustomNode` is available in the **CustomNode.tsx** file of the Customization for React.js example.

### Full customization of connections

Use **Connection.tsx** as a starting point from the [presets/classic/components](https://github.com/retejs/react-plugin/blob/next/src/presets/classic/components) directory of the plugin's source code.

```
tsimport { CustomConnection } from './CustomConnection'

render.addPreset(Presets.classic.setup({
  customize: {
    connection() {
      return CustomConnection
    }
  }
}))

```

### Full socket customization

Use **Socket.tsx** as a starting point from the [presets/classic/components](https://github.com/retejs/react-plugin/blob/next/src/presets/classic/components) directory of the plugin's source code.

```
tsimport { CustomSocket } from './CustomSocket'

render.addPreset(Presets.classic.setup({
  customize: {
    socket() {
      return CustomSocket
    }
  }
}))

```

## Customize context menu

As the context menu components utilize `styled-components`, you can customize their styles by:

```
tsimport styled from "styled-components";

const { Menu, Common, Search, Item, Subitems } = Presets.contextMenu

const CustomMenu = styled(Menu)`
  width: 320px;
`
const CustomItem = styled(Item)`
  background: grey;
`

render.addPreset(Presets.contextMenu.setup({
  customize: {
    main: () => CustomMenu,
    item: () => CustomItem,
    common: () => Common,
    search: () => Search,
    subitems: () => Subitems
  }
}))

```

## Other presets

- context menu
- minimap
- reroute

Check out the complete result on the Customization for React.js example page.

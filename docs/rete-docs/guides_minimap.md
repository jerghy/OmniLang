# Minimap

> Source: [https://retejs.org/docs/guides/minimap](https://retejs.org/docs/guides/minimap)

# Minimap

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

Minimap[Plugin](https://github.com/retejs/minimap-plugin)

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-minimap-plugin

```

## Prepare nodes

Predefined node sizes are necessary for the minimap plugin as they are displayed on the minimap with their respective dimensions.

```
tsclass Node extends ClassicPreset.Node {
  width = 190;
  height = 120;
}
class Connection<N extends Node> extends ClassicPreset.Connection<N, N> {}

type Schemes = GetSchemes<Node, Connection<Node>>;

```

## Plugin connection

```
tsimport { MinimapExtra, MinimapPlugin } from "rete-minimap-plugin";

type AreaExtra =
  // ...
  | MinimapExtra

const minimap = new MinimapPlugin<Schemes>()();

area.use(minimap);

```

But this is not sufficient as the render plugin is responsible for visualization

## Rendering the minimap

Currently, the visualization of the minimap is possible using rendering plugins for **React.js**, **Vue.js**, **Angular**, **Svelte** and **Lit**.

```
tsimport { Presets } from "rete-react-plugin"; // or  rete-vue-plugin, rete-angular-plugin, rete-svelte-plugin, @retejs/lit-plugin

render.addPreset(Presets.minimap.setup({ size: 200 }));

```

For a comprehensive guide on how to connect a specific renderer plugin to your stack version, please follow the guide for
React.js, Vue.js, Angular, Svelte or Lit

The minimap is automatically inserted into your editor and has absolute positioning. If your editor is smaller than the viewport, your editor's container should have `position: relative`.

Check out the complete result on the Minimap example page.

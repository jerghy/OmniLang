# Reroute

> Source: [https://retejs.org/docs/guides/reroute](https://retejs.org/docs/guides/reroute)

# Reroute

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

RerouteSelectable[Plugin](https://github.com/retejs/connection-reroute-plugin)

## Install dependencies

Want to start faster? Use Rete Kit to create a fully configured project in minutes!Copy the command npx rete-kit app to clipboardLearn More
```
bashnpm i rete-connection-reroute-plugin

```

## Plugin connection

```
tsimport { ReroutePlugin, RerouteExtra } from "rete-connection-reroute-plugin";

type AreaExtra =
  | Area2D<Schemes> // this type is mandatory
  | ReactArea2D<Schemes>
  | RerouteExtra;

const reroutePlugin = new ReroutePlugin<Schemes>();

render.use(reroutePlugin)

```

The plugin is connected, but you need also connect a visualization preset to render the pins

## Rendering

Currently, the visualization of the reroute pins is possible using rendering plugins for **React.js**, **Vue.js**, **Angular**, **Svelte** and **Lit**.

```
tsimport { Presets } from "rete-react-plugin"; // or  rete-vue-plugin, rete-angular-plugin, rete-svelte-plugin, @retejs/lit-plugin

render.addPreset(Presets.reroute.setup({
  contextMenu(id) {
    reroutePlugin.remove(id);
  },
  translate(id, dx, dy) {
    reroutePlugin.translate(id, dx, dy);
  }
}));

```

For a comprehensive guide on how to connect a specific renderer plugin to your stack version, please follow the guide for
React.js, Vue.js, Angular, Svelte or Lit

## Selectable Pins

As explained in the Selectable guide, you can adjust the selection of all types of elements.

The following code must be used to incorporate pins to the selection system:

```
tsimport { RerouteExtensions } from "rete-connection-reroute-plugin";

const selector = AreaExtensions.selector();
const accumulating = AreaExtensions.accumulateOnCtrl();

AreaExtensions.selectableNodes(area, selector, { accumulating });
RerouteExtensions.selectablePins(reroutePlugin, selector, accumulating);

render.addPreset(Presets.reroute.setup({
  pointerdown(id) {
    reroutePlugin.unselect(id);
    reroutePlugin.select(id);
  },
  // keep contextMenu and translate from the code above
}));

```

where

- `RerouteExtensions.selectablePins` is a compact extension that adds or removes pins to/from the registry of selected elements, and enables their movement.
- `pointerdown` event is triggered upon clicking a pin and designates it as the selected pin

Check out the complete result on the Reroute example page.

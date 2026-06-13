# Selectable

> Source: [https://retejs.org/docs/guides/selectable](https://retejs.org/docs/guides/selectable)

# Selectable

This guide is based on the Basic guide. It is recommended to review it for a comprehensive understanding of this guide.

BasicLasso/marquee selectionConnectionsCommentsReroute[Area Plugin](https://github.com/retejs/area-plugin)

## Selectable nodes

As explained in the Basic guide, you can enable node selection by using the `selectableNodes` extension

```
tsconst selector = AreaExtensions.selector()
const accumulating = AreaExtensions.accumulateOnCtrl()

AreaExtensions.selectableNodes(area, selector, { accumulating });

```

The code indicates that users can select multiple nodes by holding down the Ctrl key, then these nodes can be moved together

## Selection or deselecting

In addition to user actions, a node can be selected or deselected through the built-in methods of `selectableNodes`

```
tsconst selectableNodes = AreaExtensions.selectableNodes(area, selector, { accumulating });

selectableNodes.select(nodeId) // select a single node, the previous selection will be cleared
selectableNodes.select(nodeId, true) // select a node without clearing previous selections
selectableNodes.unselect(nodeId) // remove the node from the selected li

```

## Selectable custom elements

All elements added to the area can be added to the selector. They can act like nodes: can be selected and moved alongside other elements that are currently selected

Let's take a look at an example of adding an element to the selector

```
tsconst id = 'element-id'
const label = 'element-type'

selector.add({
  id,
  label,
  translate(dx, dy) {
    // change position of current element by dx,dy
  },
  unselect() {
    // triggered when removed from selector
    // here you can trigger styles updating
  },
}, accumulating.active())

```

Once this step is completed, the `translate` function will be called every time a selected node or other element is moved.

Before making other selected elements move with the element being dragged, you need to mark the element that the user is directly interacting with.

```
tsselector.pick({ id, label })

```

When an event such as `pointermove` is triggered on your element, it's important to verify that it is a grabbed element, and then apply the offset to all the others.

```
tsif (selector.isPicked({ id, label })) selector.translate(dx, dy)

```

where `dx`, `dy` is an offset of your element within the area's coordinates. Keep in mind that if `transform.k` isn't equal to 1, the values will deviate from the screen coordinates.

This `pick` + `isPicked` approach prevents looping when calling `selector.translate` not only on the `pointermove` event, but also on any position changes of the element through other means.

Removing an element from the selector can be easily achieved by:

```
tsselector.remove({ id, label })

```

## Extend selector

Apart from the `AreaExtensions.selector()` function, you have the option to directly use the `AreaExtensions.Selector` class.

```
tsconst selector = new AreaExtensions.Selector()

```

The benefit of using a class-based selector is that it can be expanded and customized to include the functionalities you need, like tracking the selected or unselected elements

```
tsimport type { SelectorEntity } from 'rete-area-plugin/_types/extensions/selectable'

class MySelector<E extends SelectorEntity> extends AreaExtensions.Selector<E> {
  add(entity: E, accumulate: boolean): void {
    super.add(entity, accumulate)
    console.log('added', entity)
  }
  remove(entity: E): void {
    super.remove(entity)
    console.log('removed', entity)
  }
}

```

## Other use cases

- Connections
- Comments
- Reroute

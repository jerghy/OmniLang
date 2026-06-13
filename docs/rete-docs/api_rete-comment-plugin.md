# API for rete-comment-plugin package

> Source: [https://retejs.org/docs/api/rete-comment-plugin](https://retejs.org/docs/api/rete-comment-plugin)

# API for rete-comment-plugin package

## Produces

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L20)

A union of all possible signals that can be emitted by the comment plugin

```
tstype Produces =
  | { data: Comment; type: "commentcreated" }
  | { data: Comment; type: "commentremoved" }
  | { data: Comment; type: "editcomment" }
  | { data: Comment; type: "commentselected" }
  | { data: Comment; type: "commentunselected" }
  | { data: Object; type: "commenttranslated" }
  | { data: Object; type: "commentlinktranslate" };

```

## CommentPlugin

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L41)

A plugin that provides comments for nodes

```
tsclass CommentPlugin<Schemes extends ExpectedSchemes, K extends unknown>

```

| Parameter | Extends | Description |
| --- | --- | --- |
| Schemes | ExpectedSchemes |  |
| K | unknown |  |

**Extends** Scope<Produces, BaseArea<Schemes> | K>

### constructor

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L50)

```
tsconstructor(props: Props): CommentPlugin<Schemes, K>

```

| Parameter | Type | Description |
| --- | --- | --- |
| props | Props | Optional comment plugin properties |

**Returns** `CommentPlugin<Schemes, K>`

### addFrame

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L202)

Adds a frame comment. Represents a rectangle with a text and nodes linked to it.
When user translates a comment, all linked nodes will be translated as well.
When user drops a node on a comment, the node will be linked to the comment.

```
tsaddFrame(text: string, links: string[]): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| text | string | Comment text |
| links | string[] | List of node IDs the comment is linked with |

**Returns** `void`

### addInline

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L178)

Adds an inline comment which is represented by a block with text at certain position, which is attached to the node.
When user translates a node, the comment will be translated as well.
When user drops a comment on a node, the comment will be linked to the node.

```
tsaddInline(text: string, position: [number, number], link: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| text | string | Comment text |
| position | [number, number] | Comment position |
| link | string | Node ID the comment is linked with |

**Returns** `void`

### clear

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L286)

Removes all comments

```
tsclear(): void

```

**Returns** `void`

### delete

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L230)

Removes a comment

```
tsdelete(id: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Comment id |

**Returns** `void`

### editComment

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L157)

Trigger edit form for a comment

```
tseditComment(id: string): Promise<void>

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Comment id |

**Returns** `Promise<void>`

### select

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L261)

Selects a comment

```
tsselect(id: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Comment id |

**Returns** `void`

### translate

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L249)

Translates a comment

```
tstranslate(id: string, dx: number, dy: number): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Comment id |
| dx | number | Delta x |
| dy | number | Delta y |

**Returns** `void`

### unselect

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L274)

Unselects a comment

```
tsunselect(id: string): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| id | string | Comment id |

**Returns** `void`

## CommentExtensions

[extensions/index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/extensions/index.ts#L1)

Comment plugin extensions

### selectable

[extensions/selectable.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/extensions/selectable.ts#L13)

Enables synchronization between comments and the selector

```
tsselectable(plugin: CommentPlugin<S, K>, selector: Selector<SelectorEntity>, accumulating: { active: unknown }): void

```

| Parameter | Type | Description |
| --- | --- | --- |
| plugin | CommentPlugin<S, K> | Comment plugin instance |
| selector | Selector<SelectorEntity> | Selector instance |
| accumulating | { active: unknown } | Accumulating state |

**Returns** `void`

## Props

[index.ts](https://github.com/retejs/comment-plugin/blob/51a5da1/src/index.ts#L32)

Comment plugin properties

```
tstype Props = { edit: Function };

```

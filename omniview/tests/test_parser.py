"""Tests for the OmniView parser."""
import pytest
from omniview.parser import OmniViewTransformer
from omniview.ast_nodes import (
    Program, ComponentDef, StoreDef, ThemeDef,
    PropDecl, SignalDecl, ComputedDecl, EventDecl,
    Element, IfDirective, ForDirective, ViewBlock,
    TextNode, Interpolation, InlineFn, FnRef,
    LifecycleHook, StyleBlock, StyleRule, FnDef,
    VarDeclStmt, AssignStmt, ReturnStmt, IfStmt,
)


@pytest.fixture(scope="module")
def parser():
    """Create parser via unified OmniLang lexer entry point."""
    from omnilang.lexer import create_omniview_parser
    return create_omniview_parser()


@pytest.fixture
def transformer():
    return OmniViewTransformer()


def parse(parser, transformer, source: str) -> Program:
    tree = parser.parse(source)
    return transformer.transform(tree)


# ── Basic Parsing ─────────────────────────────────────

class TestBasicParsing:
    """Minimal parsing tests."""

    def test_empty_component(self, parser, transformer):
        src = "component Empty { view { <Container /> } }"
        prog = parse(parser, transformer, src)
        assert len(prog.components) == 1
        assert prog.components[0].name == "Empty"
        assert prog.components[0].view is not None

    def test_component_with_props(self, parser, transformer):
        src = """component Greeting {
            prop name: String = "World";
            prop age: i32;
            view { <Text>{name}</Text> }
        }"""
        prog = parse(parser, transformer, src)
        comp = prog.components[0]
        assert len(comp.props) == 2
        assert comp.props[0].name == "name"
        assert comp.props[1].name == "age"

    def test_component_with_signals(self, parser, transformer):
        src = """component Counter {
            signal count: i32 = 0;
            signal name: String = "";
            view { <Text>{count}</Text> }
        }"""
        prog = parse(parser, transformer, src)
        comp = prog.components[0]
        assert len(comp.signals) == 2


class TestStoreParsing:
    """Store definition tests."""

    def test_store_with_signals(self, parser, transformer):
        src = """store AppStore {
            signal count: i32 = 0;
            signal user: String = "";
        }"""
        prog = parse(parser, transformer, src)
        assert len(prog.stores) == 1
        store = prog.stores[0]
        assert store.name == "AppStore"
        assert len(store.signals) == 2

    def test_store_with_getter(self, parser, transformer):
        src = """store AppStore {
            signal count: i32 = 0;
            getter doubled: i32 = count;
        }"""
        prog = parse(parser, transformer, src)
        store = prog.stores[0]
        assert len(store.getters) == 1

    def test_store_with_action(self, parser, transformer):
        src = """store AppStore {
            signal count: i32 = 0;
            action increment() {
                count = count + 1;
            }
        }"""
        prog = parse(parser, transformer, src)
        store = prog.stores[0]
        assert len(store.actions) == 1
        assert store.actions[0].name == "increment"


class TestViewParsing:
    """View tree parsing tests."""

    def test_simple_element(self, parser, transformer):
        src = """component Test {
            view { <Container><Text>Hello</Text></Container> }
        }"""
        prog = parse(parser, transformer, src)
        view = prog.components[0].view
        assert len(view.children) == 1
        elem = view.children[0]
        assert isinstance(elem, Element)
        assert elem.tag == "Container"

    def test_self_closing_element(self, parser, transformer):
        src = """component Test {
            view { <Input placeholder="Name" /> }
        }"""
        prog = parse(parser, transformer, src)
        view = prog.components[0].view
        assert isinstance(view.children[0], Element)

    def test_event_handler(self, parser, transformer):
        src = """component Test {
            view { <Button onPress={fn() { count += 1; }}>Click</Button> }
        }"""
        prog = parse(parser, transformer, src)
        view = prog.components[0].view
        elem = view.children[0]
        assert elem.tag == "Button"

    def test_interpolation(self, parser, transformer):
        src = """component Test {
            signal name: String = "World";
            view { <Text>Hello {name}</Text> }
        }"""
        prog = parse(parser, transformer, src)
        view = prog.components[0].view
        elem = view.children[0]
        assert elem.tag == "Text"


class TestDirectives:
    """@directive parsing tests."""

    def test_for_directive(self, parser, transformer):
        src = """component Test {
            signal items: Array<String> = [];
            view {
                @for item in items {
                    <Text>{item}</Text>
                }
                @empty {
                    <Text>No items</Text>
                }
            }
        }"""
        prog = parse(parser, transformer, src)
        view = prog.components[0].view
        assert len(view.children) == 1
        assert isinstance(view.children[0], ForDirective)

    def test_if_directive(self, parser, transformer):
        src = """component Test {
            signal show: bool = true;
            view {
                @if show {
                    <Text>Visible</Text>
                }
            }
        }"""
        prog = parse(parser, transformer, src)
        view = prog.components[0].view
        assert isinstance(view.children[0], IfDirective)


class TestLifecycle:
    """Lifecycle hook tests."""

    def test_mount_hook(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            @mount { count = 10; }
            view { <Text>{count}</Text> }
        }"""
        prog = parse(parser, transformer, src)
        comp = prog.components[0]
        assert len(comp.lifecycles) == 1
        assert comp.lifecycles[0].kind == "mount"


class TestStyleBlock:
    """Style block tests."""

    def test_style_rule(self, parser, transformer):
        src = """component Test {
            view { <Text class={style.title}>Hi</Text> }
            style {
                .title {
                    layout: column;
                    gap: 16px;
                }
            }
        }"""
        prog = parse(parser, transformer, src)
        comp = prog.components[0]
        assert comp.style is not None
        assert len(comp.style.rules) == 1


class TestThemeDef:
    """Theme definition tests."""

    def test_theme(self, parser, transformer):
        src = """theme {
            colors {
                primary: "#1890ff";
                surface: "#ffffff";
            }
        }"""
        prog = parse(parser, transformer, src)
        assert len(prog.themes) == 1
        assert len(prog.themes[0].categories) == 1

    def test_theme_multiple_categories(self, parser, transformer):
        src = """theme {
            colors { primary: "#1890ff"; accent: "#52c41a"; }
            spacing { sm: "8px"; md: "16px"; }
            radius { md: "8px"; }
        }"""
        prog = parse(parser, transformer, src)
        assert len(prog.themes) == 1


class TestComplexComponents:
    def test_multiple_stores(self, parser, transformer):
        src = """store A { signal x: i32 = 0; }
        store B { signal y: String = ""; }
        component Test { @store A; @store B; view { <Text>Hi</Text> } }"""
        prog = parse(parser, transformer, src)
        assert len(prog.stores) == 2
        assert len(prog.components[0].stores) == 2

    def test_component_with_computed(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            computed doubled: i32 = count * 2;
            view { <Text>Hi</Text> }
        }"""
        prog = parse(parser, transformer, src)
        assert prog.components[0].computeds[0].name == "doubled"

    def test_fn_with_params(self, parser, transformer):
        src = """component Test {
            fn greet(name: String) -> String { return name; }
            view { <Text>Hi</Text> }
        }"""
        prog = parse(parser, transformer, src)
        fn = prog.components[0].fns[0]
        assert fn.name == "greet"
        assert len(fn.params) == 1

    def test_unmount_hook(self, parser, transformer):
        src = """component Test {
            @unmount { }
            view { <Text>Hi</Text> }
        }"""
        prog = parse(parser, transformer, src)
        assert len(prog.components[0].lifecycles) == 1

    def test_store_with_async_action(self, parser, transformer):
        src = """store Data {
            signal items: Array<String> = [];
            async action fetch() { items = []; }
        }
        component Test { view { <Text>Hi</Text> } }"""
        prog = parse(parser, transformer, src)
        assert prog.stores[0].actions[0].name == "fetch"

    def test_error_boundary(self, parser, transformer):
        src = """component Safe {
            view {
                @error(fallback) {
                    <Text>Error!</Text>
                }
            }
        }"""
        prog = parse(parser, transformer, src)
        assert prog.components[0].view is not None
        # ErrorBoundary should be in the view's children
        children = prog.components[0].view.children
        from omniview.ast_nodes import ErrorBoundary
        error_nodes = [c for c in children if isinstance(c, ErrorBoundary)]
        assert len(error_nodes) == 1
        assert error_nodes[0].name == "fallback"

    def test_ref_decl_and_prop(self, parser, transformer):
        src = """component FocusInput {
            ref inputRef: String;
            view {
                <Input ref={inputRef} placeholder="Focus me" />
            }
        }"""
        prog = parse(parser, transformer, src)
        comp = prog.components[0]
        assert len(comp.refs) == 1
        assert comp.refs[0].name == "inputRef"
        # The element should have a ref prop
        el = comp.view.children[0]
        from omniview.ast_nodes import RefProp
        ref_props = [p for p in el.props if isinstance(p.value, RefProp)]
        assert len(ref_props) == 1
        assert ref_props[0].value.ref_name == "inputRef"

    def test_generic_component(self, parser, transformer):
        src = """component ListView<T> {
            prop items: Array<T> = [];
            view {
                @for item in items {
                    <ListItem key={item}>{item}</ListItem>
                }
            }
        }"""
        prog = parse(parser, transformer, src)
        comp = prog.components[0]
        assert comp.name == "ListView"
        assert comp.generic_params == ("T",)

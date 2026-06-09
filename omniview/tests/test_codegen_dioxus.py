"""Tests for OmniView Dioxus codegen."""
import pytest

from omniview.parser import OmniViewTransformer
from omniview.semantic.analyzer import SemanticAnalyzer
from omniview.lowering import IRFixture
from omniview.codegen.dioxus import DioxusCodegen


@pytest.fixture(scope="module")
def parser():
    from omnilang.lexer import create_omniview_parser
    return create_omniview_parser()


@pytest.fixture
def transformer():
    return OmniViewTransformer()


def generate(parser, transformer, source: str) -> dict[str, str]:
    tree = parser.parse(source)
    prog = transformer.transform(tree)
    SemanticAnalyzer().analyze(prog)
    ir_map = IRFixture().lower_program(prog)
    codegen = DioxusCodegen()
    all_files = {}
    for name, ir in ir_map.items():
        if "states" in ir or "view" in ir:
            result = codegen.generate_component(ir, ir_map)
        elif "signals" in ir and "view" not in ir:
            result = codegen.generate_store(ir)
        else:
            continue
        all_files.update(result.files)
    return all_files


class TestDioxusCodegen:
    def test_generates_use_signal(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert "use_signal" in rs
        assert "use dioxus::prelude::*" in rs

    def test_generates_rsx(self, parser, transformer):
        src = """component Test {
            view { <Container><Text>Hello</Text></Container> }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert "rsx!" in rs
        assert "div {" in rs
        assert "span {" in rs

    def test_generates_event_handler(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn increment() { count = count + 1; }
            view { <Button onPress={increment}>Click</Button> }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert "onclick:" in rs
        assert "move |" in rs

    def test_generates_bind_value(self, parser, transformer):
        src = """component Test {
            signal email: String = "";
            view { <Input bind:value={email} /> }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert "oninput:" in rs
        assert ".set(e.value())" in rs

    def test_generates_conditional(self, parser, transformer):
        src = """component Test {
            signal show: bool = true;
            view { @if show { <Text>Visible</Text> } }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert "if " in rs

    def test_generates_list(self, parser, transformer):
        src = """component Test {
            signal items: Array<String> = [];
            view { @for item in items { <ListItem key={item}>{item}</ListItem> } }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert ".iter()" in rs
        assert ".map(|" in rs

    def test_generates_store(self, parser, transformer):
        src = """store Counter {
            signal count: i32 = 0;
            getter display: i32 = count;
            action increment() { count = count + 1; }
        }
        component Test {
            @store Counter;
            view { <Text>{store.display}</Text> }
        }"""
        files = generate(parser, transformer, src)
        store = files.get("counter_store.rs", "")
        assert "GlobalSignal" in store
        assert "#[derive(Clone, PartialEq)]" in store
        assert "state.count" in store  # Getter should use state. prefix

    def test_component_structure(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        rs = files.get("Test.rs", "")
        assert "#[component]" in rs
        assert "pub fn Test() -> Element" in rs

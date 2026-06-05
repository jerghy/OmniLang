"""Tests for the OmniView semantic analyzer."""
import pytest
from lark import Lark
from omniview.parser import OmniViewTransformer
from omniview.semantic.analyzer import SemanticAnalyzer


@pytest.fixture(scope="module")
def parser():
    import os
    grammar_path = os.path.join(os.path.dirname(__file__), "..", "grammar.lark")
    with open(grammar_path, "r", encoding="utf-8") as f:
        grammar = f.read()
    return Lark(grammar, parser="earley", start="start")


@pytest.fixture
def transformer():
    return OmniViewTransformer()


def analyze(parser, transformer, source: str):
    tree = parser.parse(source)
    prog = transformer.transform(tree)
    return SemanticAnalyzer().analyze(prog)


class TestSymbolResolution:
    def test_resolves_component_symbols(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            computed doubled: i32 = count * 2;
            fn greet() -> String { return "hi"; }
            view { <Text>{count}</Text> }
        }"""
        result = analyze(parser, transformer, src)
        symbols = {k: v.kind for k, v in result.symbols.items()}
        assert "count" in symbols
        assert symbols["count"] == "signal"
        assert "doubled" in symbols
        assert symbols["doubled"] == "computed"
        assert "greet" in symbols
        assert symbols["greet"] == "fn"

    def test_resolves_store_symbols(self, parser, transformer):
        src = """store AppStore {
            signal todos: Array<Todo> = [];
            getter count: i32 = 0;
            action add() { }
        }"""
        result = analyze(parser, transformer, src)
        symbols = {k: v.kind for k, v in result.symbols.items()}
        assert "store.AppStore.todos" in symbols
        assert symbols["store.AppStore.todos"] == "store_signal"

    def test_reports_unresolved_reference(self, parser, transformer):
        src = """component Test {
            view { <Text>{unknown_var}</Text> }
        }"""
        result = analyze(parser, transformer, src)
        errors = [d for d in result.diagnostics if d.level == "ERROR"]
        assert len(errors) >= 1
        assert any("unknown_var" in e.message for e in errors)


class TestConstraintValidation:
    def test_p0214_bind_on_prop(self, parser, transformer):
        src = """component Test {
            prop name: String;
            view { <Input bind:value={name} /> }
        }"""
        result = analyze(parser, transformer, src)
        p0214 = [d for d in result.diagnostics if d.code == "P0214"]
        assert len(p0214) >= 1

    def test_p0214_bind_on_signal_ok(self, parser, transformer):
        src = """component Test {
            signal email: String = "";
            view { <Input bind:value={email} /> }
        }"""
        result = analyze(parser, transformer, src)
        p0214 = [d for d in result.diagnostics if d.code == "P0214"]
        assert len(p0214) == 0

    def test_p0201_for_without_key(self, parser, transformer):
        src = """component Test {
            signal items: Array<String> = [];
            view { @for item in items { <Text>{item}</Text> } }
        }"""
        result = analyze(parser, transformer, src)
        p0201 = [d for d in result.diagnostics if d.code == "P0201"]
        assert len(p0201) >= 1

    def test_p0201_for_with_key_ok(self, parser, transformer):
        src = """component Test {
            signal items: Array<String> = [];
            view { @for item in items { <ListItem key={item}>{item}</ListItem> } }
        }"""
        result = analyze(parser, transformer, src)
        p0201 = [d for d in result.diagnostics if d.code == "P0201"]
        assert len(p0201) == 0


class TestDependencyCollection:
    def test_computed_deps(self, parser, transformer):
        src = """component Test {
            signal a: i32 = 0;
            signal b: i32 = 1;
            computed sum: i32 = a + b;
            view { <Text>{sum}</Text> }
        }"""
        result = analyze(parser, transformer, src)
        deps = result.dependencies.get("sum", [])
        assert "a" in deps
        assert "b" in deps

    def test_effect_deps(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            @effect(count) { }
            view { <Text>{count}</Text> }
        }"""
        result = analyze(parser, transformer, src)
        assert len(result.dependencies) > 0

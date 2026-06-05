"""Tests for OmniView Flutter L1 codegen."""
import pytest
from lark import Lark
from omniview.parser import OmniViewTransformer
from omniview.semantic.analyzer import SemanticAnalyzer
from omniview.lowering import IRFixture
from omniview.codegen.flutter import FlutterCodegen


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


def generate(parser, transformer, source: str) -> dict[str, str]:
    tree = parser.parse(source)
    prog = transformer.transform(tree)
    SemanticAnalyzer().analyze(prog)
    ir_map = IRFixture().lower_program(prog)
    codegen = FlutterCodegen()
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


class TestFlutterCodegen:
    def test_stateful_widget(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>Hi</Text> }
        }"""
        files = generate(parser, transformer, src)
        dart = files.get("Test.dart", "")
        assert "StatefulWidget" in dart
        assert "createState" in dart
        assert "int _count = 0" in dart

    def test_setstate_in_handler(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn increment() { count = count + 1; }
            view { <Button onPress={increment}>+</Button> }
        }"""
        files = generate(parser, transformer, src)
        dart = files.get("Test.dart", "")
        assert "ElevatedButton" in dart
        assert "onPressed:" in dart

    def test_bind_value_creates_controller(self, parser, transformer):
        src = """component Test {
            signal email: String = "";
            view { <Input bind:value={email} /> }
        }"""
        files = generate(parser, transformer, src)
        dart = files.get("Test.dart", "")
        assert "TextEditingController" in dart
        assert "TextField" in dart
        assert "dispose" in dart

    def test_flex_row_column(self, parser, transformer):
        src = """component Test {
            view {
                <Flex direction="row" gap="16px">
                    <Text>A</Text>
                    <Text>B</Text>
                </Flex>
            }
        }"""
        files = generate(parser, transformer, src)
        dart = files.get("Test.dart", "")
        assert "Row(" in dart
        assert "spacing: 16" in dart

    def test_if_conditional(self, parser, transformer):
        src = """component Test {
            signal show: bool = true;
            view { @if show { <Text>Visible</Text> } }
        }"""
        files = generate(parser, transformer, src)
        dart = files.get("Test.dart", "")
        assert "if (" in dart

    def test_change_notifier_store(self, parser, transformer):
        src = """store Counter {
            signal count: i32 = 0;
            action increment() { count = count + 1; }
        }
        component Test {
            @store Counter;
            view { <Text>Hi</Text> }
        }"""
        files = generate(parser, transformer, src)
        store = files.get("counter_store.dart", "")
        assert "ChangeNotifier" in store
        assert "notifyListeners" in store

    def test_l1_comment_present(self, parser, transformer):
        """L1 code should include optimization hints"""
        src = """component Test {
            signal items: Array<String> = [];
            view { @for item in items { <ListItem key={item}>{item}</ListItem> } }
        }"""
        files = generate(parser, transformer, src)
        dart = files.get("Test.dart", "")
        assert "L2 optimization" in dart

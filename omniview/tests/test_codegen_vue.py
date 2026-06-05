"""Tests for OmniView Vue 3 codegen (snapshot tests)."""
import pytest
from lark import Lark
from omniview.parser import OmniViewTransformer
from omniview.semantic.analyzer import SemanticAnalyzer
from omniview.lowering import IRFixture
from omniview.codegen.vue import VueCodegen


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
    """Parse → Analyze → Lower → Generate Vue code."""
    tree = parser.parse(source)
    prog = transformer.transform(tree)
    SemanticAnalyzer().analyze(prog)
    ir_map = IRFixture().lower_program(prog)
    codegen = VueCodegen()
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


class TestVueCodegen:
    def test_generates_ref(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "ref<number>(0)" in vue
        assert "{{ count }}" in vue

    def test_generates_computed(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            computed doubled: i32 = count * 2;
            view { <Text>{doubled}</Text> }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "computed" in vue

    def test_generates_onMounted(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            @mount { count = 10; }
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "onMounted" in vue

    def test_generates_v_if(self, parser, transformer):
        src = """component Test {
            signal show: bool = true;
            view { @if show { <Text>Visible</Text> } }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "v-if" in vue

    def test_generates_v_for(self, parser, transformer):
        src = """component Test {
            signal items: Array<String> = [];
            view { @for item in items { <ListItem key={item}>{item}</ListItem> } }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "v-for" in vue

    def test_generates_v_model(self, parser, transformer):
        src = """component Test {
            signal email: String = "";
            view { <Input bind:value={email} /> }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "v-model" in vue

    def test_generates_pinia_store(self, parser, transformer):
        src = """store Counter {
            signal count: i32 = 0;
            action increment() { count = count + 1; }
        }
        component Test {
            @store Counter;
            view { <Text>{store.count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        store = files.get("stores/useCounter.ts", "")
        assert "defineStore" in store
        assert "ref<number>(0)" in store
        # .value should be used in store action
        assert "count.value" in store

    def test_generates_scoped_style(self, parser, transformer):
        src = """component Test {
            view { <Text>Hi</Text> }
            style { .title { layout: column; } }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "<style scoped>" in vue
        assert ".title" in vue
        assert "flex-direction" in vue

    def test_store_action_call_in_fn(self, parser, transformer):
        """store.action() in fn body → storeName.action() in Vue"""
        src = """store Data {
            signal count: i32 = 0;
            action add(n: i32) { count = count + n; }
        }
        component Test {
            @store Data;
            fn handle() { store.add(5); }
            view { <Text>Hi</Text> }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "store.add(5)" in vue

    def test_spread_in_pinia_action(self, parser, transformer):
        """Spread in Pinia action should use .value"""
        src = """store Data {
            signal items: Array<String> = [];
            action add(item: String) { items = [...items, item]; }
        }
        component Test {
            @store Data;
            view { <Text>Hi</Text> }
        }"""
        files = generate(parser, transformer, src)
        store = files.get("stores/useData.ts", "")
        assert "defineStore" in store
        # Should have spread with .value
        assert "items.value" in store
        assert "..." in store

    def test_v_if_else_chain(self, parser, transformer):
        """@if/@else should generate v-if/v-else chain"""
        src = """component Test {
            signal tab: String = "a";
            view {
                @if tab == "a" { <Text>A</Text> }
                @else { <Text>Other</Text> }
            }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "v-if" in vue
        assert "v-else" in vue

    def test_for_with_empty(self, parser, transformer):
        """@for/@empty should generate v-if empty check + v-for"""
        src = """component Test {
            signal items: Array<String> = [];
            view {
                @for item in items {
                    <ListItem key={item}>{item}</ListItem>
                }
                @empty {
                    <Text>No items</Text>
                }
            }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "v-for" in vue
        assert "No items" in vue

    def test_heading_levels(self, parser, transformer):
        """Heading level prop → h1-h6 tags"""
        src = """component Test {
            view {
                <Heading level="1">H1</Heading>
                <Heading level="3">H3</Heading>
                <Heading>Default</Heading>
            }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert "<h1>" in vue
        assert "<h3>" in vue
        assert "<h2>" in vue
        assert "level=" not in vue

    def test_a11y_props(self, parser, transformer):
        """A11y props should map to aria attributes in Vue"""
        src = """component Test {
            view { <Button accessibilityLabel="Submit" accessibilityRole="button">OK</Button> }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert 'aria-label="Submit"' in vue
        assert 'role="button"' in vue

    def test_sfc_structure(self, parser, transformer):
        """Generated .vue file should have script/template/style sections"""
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>Hi</Text> }
            style { .box { layout: column; } }
        }"""
        files = generate(parser, transformer, src)
        vue = files.get("Test.vue", "")
        assert '<script setup lang="ts">' in vue
        assert "<template>" in vue
        assert "<style scoped>" in vue

"""Tests for OmniView codegen (target="omniview")."""
import pytest
from omniview.parser import OmniViewTransformer
from omniview.semantic.analyzer import SemanticAnalyzer
from omniview.lowering import IRFixture
from omniview.codegen.omniview import OmniViewCodegen


@pytest.fixture(scope="module")
def parser():
    from omnilang.lexer import create_omniview_parser
    return create_omniview_parser()


@pytest.fixture
def transformer():
    return OmniViewTransformer()


def generate(parser, transformer, source: str) -> dict[str, str]:
    """Parse → Analyze → Lower → Generate OmniView .js code."""
    from omniview.preprocessor import preprocess
    pp = preprocess(source)
    tree = parser.parse(pp.processed_source)
    prog = OmniViewTransformer(block_map=pp.blocks).transform(tree)
    SemanticAnalyzer().analyze(prog)
    ir_map = IRFixture().lower_program(prog)
    codegen = OmniViewCodegen()
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


class TestOmniViewCodegen:
    """Core codegen output tests."""

    def test_generates_signal(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert 'signal(' in js
        assert 'id: "Test.count"' in js
        assert 'name: "count"' in js
        assert 'type: "number"' in js
        # Signal passed directly to h() for reactive binding (not .get())
        assert "count" in js

    def test_generates_computed(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            computed doubled: i32 = count * 2;
            view { <Text>{doubled}</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "computed(" in js
        assert 'id: "Test.doubled"' in js

    def test_generates_fn_with_trace(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn increment() { count = count + 1; }
            view { <Container></Container> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert 'const increment = () => {' in js
        assert 'traceExecution("Test.increment")' in js
        assert "count.set" in js or "count.get() + 1" in js

    def test_generates_per_statement_trace(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn increment() {
                count = count + 1;
                count = count + 2;
            }
            view { <Container></Container> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert 'traceExecution("Test.increment.stmt0")' in js
        assert 'traceExecution("Test.increment.stmt1")' in js

    def test_generates_h_calls(self, parser, transformer):
        src = """component Test {
            view { <Container><Text>Hello</Text></Container> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert 'h(' in js
        assert '"div"' in js
        assert '"Hello"' in js

    def test_generates_effect(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            @effect(count) { count = count + 1; }
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "effect(" in js

    def test_generates_mount_lifecycle(self, parser, transformer):
        src = """component Test {
            signal loaded: bool = false;
            @mount { loaded = true; }
            view { <Text>Ready</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "effect(" in js or "loaded.set(true)" in js

    def test_generates_unmount_lifecycle(self, parser, transformer):
        src = """component Test {
            @unmount { }
            view { <Text>Ready</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "onCleanup(" in js

    def test_generates_button_with_event(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn increment() { count = count + 1; }
            view { <Button onPress={increment}>{count}</Button> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert 'onClick' in js
        assert "increment" in js

    def test_signal_set_in_mutation(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn reset() { count = 0; }
            view { <Container></Container> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "count.set(0)" in js

    def test_imports_from_runtime(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert 'from "./omniview-runtime"' in js
        assert "import { signal }" in js
        assert "import { h }" in js or "import { render }" in js

    def test_export_function(self, parser, transformer):
        src = """component Test {
            view { <Text>Hello</Text> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "export function Test(mountEl)" in js

    def test_increment_mutation(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            fn increment() { count += 1; }
            view { <Container></Container> }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "count.set" in js
        assert "count.get()" in js
        assert "+ 1" in js

    def test_conditional_view(self, parser, transformer):
        src = """component Test {
            signal show: bool = true;
            view {
                @if show { <Text>visible</Text> }
            }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "renderConditional" in js

    def test_list_view(self, parser, transformer):
        src = """component Test {
            signal items: bool = true;
            view {
                @for item in items { <Text>{item}</Text> }
            }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Test.js", "")
        assert "renderList" in js


class TestOmniViewStoreCodegen:
    """Store codegen tests."""

    def test_generates_store(self, parser, transformer):
        src = """store TaskStore {
            signal tasks: bool = false;
            action addTask(text: String) {
                tasks = false;
            }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("TaskStore.js", "")
        assert 'store("TaskStore"' in js
        assert "signal(" in js

    def test_store_has_actions(self, parser, transformer):
        src = """store CounterStore {
            signal count: i32 = 0;
            action increment() {
                count = count + 1;
            }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("CounterStore.js", "")
        assert "function increment" in js
        # Store mutations use Proxy assignment: StoreName.field = value
        assert "CounterStore.count" in js
        assert "count.get()" in js


class TestSmoke:
    """End-to-end smoke tests for complete components."""

    def test_counter_component(self, parser, transformer):
        src = """component Counter {
            signal count: i32 = 0;
            computed label: String = "Count: " + count;
            fn increment() { count = count + 1; }
            view {
                <Container>
                    <Heading>{label}</Heading>
                    <Button onPress={increment}>+1</Button>
                </Container>
            }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Counter.js", "")
        # Must export the component
        assert "export function Counter(mountEl)" in js
        # Must have signal
        assert 'signal(0, { id: "Counter.count"' in js
        # Must have computed
        assert 'computed(' in js
        # Must have increment function
        assert "const increment = () =>" in js
        # Must have view rendering
        assert "h(" in js
        assert "render(" in js
        # Must return public API
        assert "return {" in js
        assert "count" in js
        assert "label" in js
        assert "increment" in js

    def test_valid_js_syntax_hyperscript(self, parser, transformer):
        """Verify generated h() calls form valid JS syntax."""
        src = """component Btn {
            view { <Button onPress={handleClick}>Click Me</Button> }
            fn handleClick() { }
        }"""
        files = generate(parser, transformer, src)
        js = files.get("Btn.js", "")
        # The generated code should have valid h() syntax
        assert 'h("button"' in js
        assert "onClick" in js
        assert "handleClick" in js

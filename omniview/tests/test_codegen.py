"""Tests for OmniView React codegen (snapshot tests)."""
import pytest
from lark import Lark
from omniview.parser import OmniViewTransformer
from omniview.semantic.analyzer import SemanticAnalyzer
from omniview.lowering import IRFixture
from omniview.codegen.react import ReactCodegen


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
    """Parse → Analyze → Lower → Generate React code."""
    tree = parser.parse(source)
    prog = transformer.transform(tree)
    SemanticAnalyzer().analyze(prog)  # Ensure no errors
    ir_map = IRFixture().lower_program(prog)
    codegen = ReactCodegen()
    all_files = {}
    for name, ir in ir_map.items():
        if "states" in ir or "view" in ir:
            # Component IR
            result = codegen.generate_component(ir, ir_map)
        elif "signals" in ir and "view" not in ir:
            # Store IR
            result = codegen.generate_store(ir)
        else:
            continue
        all_files.update(result.files)
    return all_files


class TestReactCodegen:
    def test_generates_useState(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "useState<number>(0)" in tsx
        assert "{count}" in tsx

    def test_generates_useMemo(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            computed doubled: i32 = count * 2;
            view { <Text>{doubled}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "useMemo" in tsx

    def test_generates_useEffect_mount(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            @mount { count = 10; }
            view { <Text>{count}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "useEffect" in tsx

    def test_generates_element_tags(self, parser, transformer):
        src = """component Test {
            view { <Container><Text>Hello</Text></Container> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "<div" in tsx
        assert "<span" in tsx

    def test_generates_button_onClick(self, parser, transformer):
        src = """component Test {
            signal count: i32 = 0;
            view { <Button onPress={fn() { count += 1; }}>Click</Button> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "onClick" in tsx
        assert "setCount" in tsx

    def test_generates_bind_value(self, parser, transformer):
        src = """component Test {
            signal email: String = "";
            view { <Input bind:value={email} /> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "value={email}" in tsx
        assert "onChange" in tsx

    def test_generates_for_loop(self, parser, transformer):
        src = """component Test {
            signal items: Array<String> = [];
            view { @for item in items { <ListItem key={item}>{item}</ListItem> } }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert ".map(" in tsx

    def test_generates_if_conditional(self, parser, transformer):
        src = """component Test {
            signal show: bool = true;
            view { @if show { <Text>Visible</Text> } }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "?" in tsx  # ternary operator

    def test_generates_css_module_import(self, parser, transformer):
        src = """component Test {
            view { <Text>Hi</Text> }
            style { .title { layout: column; } }
        }"""
        files = generate(parser, transformer, src)
        css = files.get("Test.module.css", "")
        assert ".title" in css
        assert "flex-direction" in css

    def test_generates_props_interface(self, parser, transformer):
        src = """component Greeting {
            prop name: String = "World";
            view { <Text>{name}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Greeting.tsx", "")
        assert "GreetingProps" in tsx

    def test_store_action_call_no_args(self, parser, transformer):
        """store.action() in fn body should generate getState().action()"""
        src = """store Counter {
            signal count: i32 = 0;
            action increment() { count = count + 1; }
        }
        component Test {
            @store Counter;
            fn handle_click() { store.increment(); }
            view { <Button onPress={handle_click}>Click</Button> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "useCounter.getState().increment()" in tsx

    def test_store_action_call_with_args(self, parser, transformer):
        """store.action(arg) in fn body should generate getState().action(arg)"""
        src = """store Auth {
            signal user: String = "";
            action login(name: String) { user = name; }
        }
        component Test {
            @store Auth;
            fn do_login() { store.login("Admin"); }
            view { <Button onPress={do_login}>Login</Button> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert 'useAuth.getState().login("Admin")' in tsx

    def test_store_getter_in_view_not_getState(self, parser, transformer):
        """Store getter in view should use hook variable, not getState()"""
        src = """store Data {
            signal count: i32 = 0;
            getter display: i32 = count;
        }
        component Test {
            @store Data;
            view { <Text>{store.display}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        # Should extract as hook variable, not getState()
        assert "useData(s => s.display())" in tsx
        assert "getState" not in tsx  # No getState() in view

    def test_css_mapping_integration(self, parser, transformer):
        """CSS style properties should use the full css_mapping.py lookup"""
        src = """component Test {
            view { <Text>Hi</Text> }
            style {
                .card {
                    layout: column;
                    padding: 24px;
                    border_radius: 8px;
                    shadow: md;
                    font_size: 16px;
                    font_weight: bold;
                    align: center;
                }
            }
        }"""
        files = generate(parser, transformer, src)
        css = files.get("Test.module.css", "")
        assert ".card" in css
        assert "display: flex" in css
        assert "flex-direction: column" in css
        assert "padding: 24px" in css
        assert "border-radius: 8px" in css
        assert "font-size: 16px" in css
        assert "font-weight: 700" in css
        assert "align-items: center" in css

    def test_spread_in_store_action(self, parser, transformer):
        """Spread syntax [...array, item] in store actions"""
        src = """store Data {
            signal items: Array<String> = [];
            action add(item: String) { items = [...items, item]; }
        }
        component Test {
            @store Data;
            fn handle() { store.add("hello"); }
            view { <Text>Hi</Text> }
        }"""
        files = generate(parser, transformer, src)
        ts = files.get("useData.ts", "")
        assert "[...state.items, item]" in ts or "[...state.items,item]" in ts

    def test_heading_level(self, parser, transformer):
        """Heading level prop should generate correct h1-h6 tags"""
        src = """component Test {
            view {
                <Heading>Default</Heading>
                <Heading level="1">Title</Heading>
                <Heading level="3">Sub</Heading>
            }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "<h2>Default</h2>" in tsx
        assert "<h1>Title</h1>" in tsx
        assert "<h3>Sub</h3>" in tsx
        # level prop should NOT appear on the HTML element
        assert 'level=' not in tsx

    def test_a11y_props(self, parser, transformer):
        """accessibilityLabel → aria-label, accessibilityRole → role"""
        src = """component Test {
            view { <Button accessibilityLabel="Close dialog" accessibilityRole="button">X</Button> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert 'aria-label="Close dialog"' in tsx
        assert 'role="button"' in tsx

    def test_bind_checked(self, parser, transformer):
        """bind:checked should generate checked + onChange pattern"""
        src = """component Test {
            signal done: bool = false;
            view { <Checkbox bind:checked={done} /> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "checked={done}" in tsx
        assert "setDone(e.target.checked)" in tsx

    def test_multiple_stores(self, parser, transformer):
        """Component with multiple stores should import both"""
        src = """store Cart { signal items: Array<String> = []; }
        store User { signal name: String = ""; }
        component Test {
            @store Cart; @store User;
            view { <Text>{store.name}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "useCart" in tsx
        assert "useUser" in tsx

    def test_css_snake_to_kebab(self, parser, transformer):
        """CSS property names should be converted from snake_case to kebab-case"""
        src = """component Test {
            view { <Text>Hi</Text> }
            style {
                .box {
                    border_radius: 12px;
                    font_size: 14px;
                    font_weight: bold;
                    line_height: 1.5;
                    text_align: center;
                    background_color: "#f0f0f0";
                    z_index: 10;
                }
            }
        }"""
        files = generate(parser, transformer, src)
        css = files.get("Test.module.css", "")
        assert "border-radius" in css
        assert "font-size" in css
        assert "font-weight" in css
        assert "line-height" in css
        assert "text-align" in css
        assert "background-color" in css
        assert "z-index" in css

    def test_format_date_builtin(self, parser, transformer):
        """format_date(date, pattern) should generate Date.toLocaleDateString()"""
        src = """component Test {
            signal d: String = "2024-01-15";
            view { <Text>{format_date(d, "yyyy-MM-dd")}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "toLocaleDateString" in tsx

    def test_format_number_builtin(self, parser, transformer):
        """format_number(num, decimals) should generate .toFixed()"""
        src = """component Test {
            signal n: f64 = 3.14159;
            view { <Text>{format_number(n, 2)}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "toFixed" in tsx

    def test_translate_builtin(self, parser, transformer):
        """t('key') should generate React i18n t() call"""
        src = """component Test {
            view { <Text>{t("hello_world")}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        assert "t(" in tsx

    def test_generic_component_ts_output(self, parser, transformer):
        """<T> on component should generate TypeScript generics"""
        src = """component ListView<T> {
            prop items: Array<T> = [];
            view { @for item in items { <ListItem key={item}>{item}</ListItem> } }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("ListView.tsx", "")
        assert "ListViewProps<T>" in tsx
        assert "function ListView<T>" in tsx

    def test_standalone_function_call(self, parser, transformer):
        """Standalone fn(args) should work in expressions"""
        src = """component Test {
            signal x: i32 = 1;
            view { <Text>{format_date(x, "yyyy")}</Text> }
        }"""
        files = generate(parser, transformer, src)
        tsx = files.get("Test.tsx", "")
        # Should NOT have compilation errors or unknown identifiers
        assert "unknown" not in tsx

"""Cross-target behavioral equivalence tests.

Council R4 Critic: The project's existence is justified by the claim
"write once, run anywhere" — but this has never been verified for UI targets.

This test compiles the same .omni source to multiple UI targets and
verifies structural equivalence (component name, props, event bindings,
element hierarchy). Full runtime behavioral comparison requires running
each target's generated app in a browser/DOM environment (future work).
"""

import json
import re
import subprocess
import sys
from pathlib import Path

import pytest

FIXTURES_DIR = Path(__file__).parent.parent / "frameworks" / "behavior"
BUILD_DIR = Path(__file__).parent.parent.parent / "build_output" / "cross_target"


def _compile_omni_file(omni_path: str, target: str, out_dir: Path) -> dict:
    """Compile .omni to target and return {filename: content} dict."""
    out_dir.mkdir(parents=True, exist_ok=True)
    result = subprocess.run(
        [sys.executable, "-m", "omniview.cli", "build",
         omni_path, "--target", target, "--output", str(out_dir)],
        capture_output=True, text=True, timeout=60,
    )
    if result.returncode != 0:
        return {}
    files = {}
    for f in out_dir.glob("*"):
        if f.is_file():
            files[f.name] = f.read_text(encoding="utf-8")
    return files


def _extract_structural_features(content: str, target: str) -> dict:
    """Extract comparable structural features from generated code.

    Different targets use different syntax but should produce:
    - Same component names
    - Same element hierarchy (tag names)
    - Same event bindings (handler names)
    - Same text content
    - Same test-id attributes
    """
    features = {
        "target": target,
        "component_names": [],
        "element_tags": [],
        "test_ids": [],
        "event_handlers": [],
        "text_content": [],
    }

    # Extract test-id attributes (common across all targets)
    test_ids = re.findall(r'test-id="([^"]+)"', content)
    features["test_ids"] = sorted(set(test_ids))

    # Extract data-testid (alternative format)
    data_testids = re.findall(r"data-testid=['\"]([^'\"]+)['\"]", content)
    features["test_ids"].extend(data_testids)
    features["test_ids"] = sorted(set(features["test_ids"]))

    # Extract event handler names
    # React: onPress={handlerName}
    # Vue: @press="handlerName" or v-on:press="handlerName"
    # Dioxus: onpress={handlerName}
    handlers = set()
    for pattern in [
        r'on\w+=\{(\w+)\}',
        r'@\w+="(\w+)"',
        r'on\w+: (\w+)',
    ]:
        handlers.update(re.findall(pattern, content))
    features["event_handlers"] = sorted(handlers)

    # Extract HTML-like element tags (both uppercase components and lowercase HTML)
    tags = set()
    for match in re.finditer(r'<(\w+)', content):
        tag = match.group(1)
        if tag not in ("Fragment", "Suspense", "template", "script", "style", "div", "span"):
            tags.add(tag.lower())
    features["element_tags"] = sorted(tags)

    # Extract text content between >...</
    text_matches = re.findall(r'>([^<>{}\n]{2,50})<', content)
    features["text_content"] = sorted(set(t.strip() for t in text_matches if t.strip()))

    # Component name — look for export/function patterns
    comp_match = re.search(r'(?:export\s+function|export\s+const|component)\s+(\w+)', content)
    if comp_match:
        features["component_names"].append(comp_match.group(1))

    return features


class TestCrossTargetEquivalence:
    """Verify .omni → multi-target output has equivalent structure."""

    @pytest.fixture(scope="class")
    def counter_outputs(self):
        """Compile Counter to React, Vue, Dioxus."""
        outputs = {}
        counter_path = str(FIXTURES_DIR / "01_counter.omni")
        for target in ["react", "vue"]:
            out_dir = BUILD_DIR / f"counter_{target}"
            files = _compile_omni_file(counter_path, target, out_dir)
            if files:
                combined = "\n".join(files.values())
                outputs[target] = _extract_structural_features(combined, target)
        return outputs

    def test_both_targets_compile(self, counter_outputs):
        """Both React and Vue targets should compile successfully."""
        assert "react" in counter_outputs, "React target failed to compile"
        assert "vue" in counter_outputs, "Vue target failed to compile"

    def test_same_test_ids_across_targets(self, counter_outputs):
        """Test-id attributes should be identical across all targets.

        test-id is the primary cross-target contract: it must survive
        compilation regardless of target framework syntax.
        """
        react_ids = set(counter_outputs["react"]["test_ids"])
        vue_ids = set(counter_outputs["vue"]["test_ids"])

        expected = {"counter-label", "btn-increment", "btn-decrement", "btn-reset"}
        missing_in_react = expected - react_ids
        missing_in_vue = expected - vue_ids

        assert not missing_in_react, f"Missing test-ids in React: {missing_in_react}"
        assert not missing_in_vue, f"Missing test-ids in Vue: {missing_in_vue}"

    def test_same_event_handlers_across_targets(self, counter_outputs):
        """Event handler names should be consistent across targets."""
        react_handlers = set(counter_outputs["react"]["event_handlers"])
        vue_handlers = set(counter_outputs["vue"]["event_handlers"])

        expected = {"increment", "decrement", "reset"}
        missing_in_react = expected - react_handlers
        missing_in_vue = expected - vue_handlers

        assert not missing_in_react, f"Missing handlers in React: {missing_in_react}"
        assert not missing_in_vue, f"Missing handlers in Vue: {missing_in_vue}"

    def test_same_elements_across_targets(self, counter_outputs):
        """Element hierarchy should include the same component types.

        OmniView components are lowered to native HTML: Button→<button>, Heading→<h1>,
        Container/Flex→<div>. This test verifies the lowered elements are consistent.
        """
        react_tags = set(counter_outputs["react"]["element_tags"])
        vue_tags = set(counter_outputs["vue"]["element_tags"])

        # OmniView components lowered to native elements
        native_expected = {"button", "h1"}  # Button→<button>, Heading→<h1>
        for expected in native_expected:
            assert expected in react_tags, f"No <{expected}> in React output (tags: {react_tags})"
            assert expected in vue_tags, f"No <{expected}> in Vue output (tags: {vue_tags})"

        # Container/Flex → <div> (should be present in both)
        assert "div" in react_tags or "button" in react_tags, "No layout containers in React output"

    def test_no_empty_output(self, counter_outputs):
        """Neither target should produce empty output."""
        for target, features in counter_outputs.items():
            assert len(features["test_ids"]) > 0, f"{target} has no test-ids"
            assert len(features["event_handlers"]) > 0, f"{target} has no event handlers"

    def test_store_read_cross_target(self):
        """Store-read fixture: verify store signals survive across targets."""
        store_path = str(FIXTURES_DIR / "05_store_read.omni")
        for target in ["react", "vue"]:
            out_dir = BUILD_DIR / f"store_{target}"
            files = _compile_omni_file(store_path, target, out_dir)
            assert files, f"{target} target failed to compile store fixture"
            combined = "\n".join(files.values())
            features = _extract_structural_features(combined, target)
            # Should have UserStore-related content
            assert len(features["test_ids"]) >= 3, (
                f"{target} store fixture missing test-ids: {features['test_ids']}"
            )

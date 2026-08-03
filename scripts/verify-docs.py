#!/usr/bin/env python3
"""Doc-compile gate: extract .omni code blocks from docs and parse-check them.

Every fenced code block in docs/omniview/*.md (16 guides) plus the BNF spec
and the v2 design doc that teaches compilable OmniLang syntax is verified
against the current parser (`omnic analyse`). The compiler is the ground
truth: parse/compile errors in a block fail the gate and must be fixed in the
DOC, never in the compiler (parser divergences are recorded as compiler bugs).

Block inclusion:
  lang=omni  — always checked.
  lang=""    — checked only when the content looks like .omni code (BNF
               productions `x ::= ...`, prose, and platform-mapping notes
               like "React: ..." are skipped — they teach no compilable
               syntax). Other langs (tsx/dart/rust/css/js/bash/toml) skipped.
  block classification:
    complete — the block is a full .omni file (starts with a top-level
               declaration). Checked as-is; ANY diagnostic error fails the
               gate (except E_IMPORT — docs teach hypothetical project
               structure whose import targets cannot exist standalone).
    fragment — everything else. Wrappers are tried in order so the block's
               SYNTAX is exercised in every plausible context:
                 1. component body  2. view body  3. fn statement body
                 4. top-level const expression
               Only genuine PARSE errors fail the gate; semantic errors
               (undeclared names etc.) are reported as notes — they follow
               from the synthetic wrapper context, not from the doc.

Usage:
  python scripts/verify-docs.py [--omnic PATH] [--check|--inspect] [-v]
  (default)  gate mode — exit 1 if any block fails (CI passes --check
             explicitly for clarity)
  --check    gate mode, explicit
  --inspect  audit mode — never exit nonzero
"""
import argparse
import json
import os
import re
import subprocess
import sys
import tempfile

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
DOCS_DIR = os.path.join(REPO_ROOT, "docs", "omniview")
SPEC_FILE = os.path.join(
    REPO_ROOT, "docs", "superpowers", "specs",
    "2026-07-31-semantic-level-dsl-syntax-spec.md",
)
DESIGN_FILE = os.path.join(
    REPO_ROOT, "docs", "superpowers", "specs",
    "2026-08-02-semantic-syntax-v2-design.md",
)
EXEMPTIONS_FILE = os.path.join(REPO_ROOT, "scripts", "doc-gate-exemptions.txt")

TOP_LEVEL_RE = re.compile(
    r"^(import\b|component\b|store\b|design\b|const\b|fn\b|struct\b|type\b|@motion\b|pub\s+(async\s+)?(fn|struct|type)\b)",
)

# Keywords whose presence marks .omni code (empty-lang blocks without any are
# reference tables / prose and are skipped).
CODE_KEYWORD_RE = re.compile(
    r"\b(import|component|store|design|const|let|pub|signal|prop|computed|event|ref|view|style|class|variant|token|action|getter|@|match|if|else|for|while|return|emit|print|try|catch|throw|struct|type)\b"
    r"|fn\s+[A-Za-z_]"
)

# Parse-error message prefixes — semantic errors (undeclared names, type
# mismatches, undefined stores) are context-dependent for wrapped fragments
# and become notes instead of failing the gate.
PARSE_ERROR_PREFIXES = (
    "Unexpected token",
    "Unexpected end",
    "Unclosed",
    "Unterminated",
    "Unbalanced",
    "Trailing",
    "Missing",
    "Invalid",
)
# Semantic type-mismatch messages also start with "expected X, got Y" —
# exclude that shape so it stays a note, not a parse failure.
SEMANTIC_EXPECTED_RE = re.compile(r"^expected [^,]+, got\b")
# Diagnoses that never fail the gate (teaching context, not syntax).
NON_FAILING_CODES = ("E_IMPORT",)


def is_parse_error(message):
    """True when a diagnostic message is a genuine PARSE error (parser
    messages use `expected X — found Y` / quoted tokens; semantic type
    mismatches use the `expected X, got Y` shape and must not fail the gate
    for wrapped fragments)."""
    if message.startswith("expected "):
        return not SEMANTIC_EXPECTED_RE.match(message)
    return message.startswith(PARSE_ERROR_PREFIXES)

BNF_RE = re.compile(r"^\s*[\w:\-']+(\s+)?::=")


def load_exemptions():
    """doc-relative-path:lineno lines with a reason — intentionally
    non-compilable teaching blocks (documented in the doc itself)."""
    exempt = set()
    if os.path.exists(EXEMPTIONS_FILE):
        with open(EXEMPTIONS_FILE, encoding="utf-8") as fh:
            for line in fh:
                line = line.strip()
                if not line or line.startswith("#"):
                    continue
                key = line.split(" ", 1)[0]
                if ":" in key:
                    exempt.add(key.replace("\\", "/"))
    return exempt


def fenced_blocks(path):
    """Yield (lineno, lang, text) for each fenced code block in `path`."""
    with open(path, encoding="utf-8") as fh:
        lines = fh.readlines()
    blocks = []
    i = 0
    n = len(lines)
    while i < n:
        m = re.match(r"^```(\w*)\s*$", lines[i].rstrip("\n"))
        if m:
            lang = m.group(1)
            start = i + 1
            i += 1
            buf = []
            while i < n and not lines[i].rstrip("\n").startswith("```"):
                buf.append(lines[i])
                i += 1
            blocks.append((start, lang, "".join(buf)))
        i += 1
    return blocks


def is_code_block(lang, text):
    """Should this fenced block be parse-checked?"""
    stripped = text.strip()
    if not stripped:
        return False
    if lang == "omni":
        return True
    if lang:
        return False  # tsx/dart/rust/css/js/bash/toml etc. — other targets
    # Empty-lang: include only content that looks like .omni code.
    if BNF_RE.match(stripped):
        return False
    if re.match(r"^(React|Flutter|OmniView|Dart|Python|JS|TS|Vue|Dioxus):", stripped):
        return False
    # Pure-operator token tables (lexical reference) are not code.
    if all(
        not re.search(r"[A-Za-z0-9一-鿿]", line)
        for line in stripped.splitlines()
        if line.strip()
    ):
        return False
    # Language-keyword reference tables (double-space separated word lists)
    # are not code.
    if re.match(r"^(\s*[a-z]+\s{2,}){3,}", stripped):
        return False
    # Prose / type-reference blocks (no code markers) are skipped.
    # Element tags are lowercase (`<div`, `<button`); `Array<String>`
    # generics and PascalCase component refs are not tag markers (blocks
    # using those carry `{`/`;`/keywords anyway). A bare `|` is not enough
    # either — type-reference tables like `i32 | String` use it.
    code_text = re.sub(r"//[^\n]*", "", stripped)  # comments can mention keywords
    if not (
        re.search(r"<[a-z]", stripped)
        or "{" in stripped
        or ";" in stripped
        or CODE_KEYWORD_RE.search(code_text)
    ):
        return False
    # Comment-only blocks teach nothing compilable.
    if all(line.strip().startswith("//") or not line.strip() for line in stripped.splitlines()):
        return False
    return True


def is_complete(text):
    for line in text.lstrip().splitlines():
        line = line.strip()
        if not line or line.startswith("//"):
            continue
        return bool(TOP_LEVEL_RE.match(line))
    return False


def wrappers_for(text):
    """Candidate renderings of a fragment, most plausible context first."""
    body = "import omniview;\n\ncomponent __DocProbe {\n" + text.rstrip() + "\n}\n"
    view = "import omniview;\n\ncomponent __DocProbe {\n    view {\n" + text.rstrip() + "\n    }\n}\n"
    fn = "fn __probe() {\n" + text.rstrip() + "\n}\n"
    expr = "const __probe = (\n" + text.rstrip() + "\n);\n"
    return [body, view, fn, expr]


def run_omnic(omnic, tmpfile, tmpdir):
    proc = subprocess.run(
        [omnic, "analyse", os.path.basename(tmpfile), "--format", "json"],
        cwd=tmpdir,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        timeout=60,
    )
    if not proc.stdout.strip():
        return {"success": False, "diagnostics": [{"severity": "error", "message": "no output"}]}
    try:
        return json.loads(proc.stdout)
    except json.JSONDecodeError:
        return {"success": False, "diagnostics": [{"severity": "error", "message": proc.stdout[:200]}]}


def check_block(omnic, tmpdir, content, fragment=False):
    """Try one rendering of a block. Returns (ok, hard_errors, notes).

    For fragments, only genuine PARSE errors fail (semantic errors follow
    from the synthetic wrapper context); complete blocks fail on any error
    except E_IMPORT (docs teach hypothetical project structure)."""
    hard = []
    notes = []
    with open(os.path.join(tmpdir, "probe.omni"), "w", encoding="utf-8") as fh:
        fh.write(content)
    result = run_omnic(omnic, "probe.omni", tmpdir)
    for d in result.get("diagnostics", []):
        if d.get("severity") != "error":
            continue
        if d.get("code") in NON_FAILING_CODES:
            notes.append(d)
        elif fragment and not is_parse_error(d.get("message", "")):
            notes.append(d)
        else:
            hard.append(d)
    return not hard, hard, notes


def audit(omnic, paths, verbose):
    failures = []
    notes = []
    checked = 0
    exempted = load_exemptions()
    with tempfile.TemporaryDirectory() as tmpdir:
        for path in paths:
            rel = os.path.relpath(path, REPO_ROOT)
            for lineno, lang, text in fenced_blocks(path):
                if not is_code_block(lang, text):
                    continue
                text = text.strip()
                if not text:
                    continue
                if f"{rel.replace(os.sep, '/')}:{lineno}" in exempted:
                    continue
                checked += 1
                complete = is_complete(text)
                # Complete-first: check the block as a full file. On failure,
                # fall back to wrapper contexts ONLY when the errors are
                # semantic (undeclared names etc.) — a block starting with a
                # top-level declaration that references signals from prose
                # context is a continuation example (semantic errors →
                # notes). A PARSE error in a complete-looking block fails
                # hard: the view-text parser is lenient and would otherwise
                # swallow broken syntax.
                ok, hard, soft = check_block(omnic, tmpdir, text)
                used_kind = "complete" if complete else "fragment"
                may_fallback = not complete or not any(
                    is_parse_error(d.get("message", "")) for d in hard
                )
                if not ok and may_fallback:
                    for wrap in wrappers_for(text):
                        o, h, s = check_block(omnic, tmpdir, wrap, fragment=True)
                        if o:
                            ok = True
                            used_kind = "fragment"
                            notes.extend((rel, lineno, "fragment", d) for d in s)
                            break
                # E_LINT "undefined store" that follows an unresolvable import
                # is downstream of the hypothetical project structure.
                if not ok and any(
                    d.get("code") == "E_IMPORT" for d in hard + soft
                ) and any(
                    "references undefined store" in d.get("message", "") for d in hard
                ):
                    soft = [d for d in hard if "references undefined store" in d.get("message", "")]
                    hard = [d for d in hard if d not in soft]
                    if not hard:
                        ok = True
                notes.extend((rel, lineno, used_kind, d) for d in soft)
                if not ok:
                    failures.append((rel, lineno, used_kind, hard))
                if verbose and (not ok or soft):
                    tag = "complete" if complete else "fragment"
                    for d in hard + soft:
                        print(f"  [{rel}:{lineno} {tag}] {d.get('message')} ({d.get('code','')})")
    return checked, failures, notes


def main():
    default_omnic = os.path.join(
        REPO_ROOT,
        "target", "release" if os.environ.get("CI") else "debug",
        "omnic.exe" if sys.platform == "win32" else "omnic",
    )
    ap = argparse.ArgumentParser()
    ap.add_argument("--omnic", default=default_omnic)
    ap.add_argument("--docs", default=DOCS_DIR)
    ap.add_argument("--check", action="store_true", help="exit 1 on failure (gate mode)")
    ap.add_argument("--inspect", action="store_true", help="never exit nonzero (audit mode)")
    ap.add_argument("--verbose", "-v", action="store_true")
    args = ap.parse_args()

    args.omnic = os.path.abspath(args.omnic)

    paths = sorted(
        os.path.join(args.docs, f) for f in os.listdir(args.docs) if f.endswith(".md")
    )
    for extra in (SPEC_FILE, DESIGN_FILE):
        if os.path.exists(extra):
            paths.append(extra)

    checked, failures, notes = audit(args.omnic, paths, args.verbose)

    print(f"\n=== doc-compile gate: {checked} omni blocks checked "
          f"({len(failures)} failed, {len(notes)} semantic notes) ===")
    for rel, lineno, kind, errs in failures:
        print(f"FAIL [{rel}:{lineno} {kind}]")
        for e in errs:
            print(f"  -> {e.get('message')} ({e.get('code', '')})")
    if args.verbose or args.inspect:
        for rel, lineno, kind, d in notes:
            print(f"NOTE [{rel}:{lineno} {kind}] {d.get('message')} ({d.get('code','')})")
    # Gate mode is the default: failures exit nonzero unless --inspect
    # (audit mode) explicitly opts out.
    if failures and not args.inspect:
        print("\nGATE FAILED: fix the DOC blocks above (compiler is ground truth).")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())

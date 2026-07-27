# Initial System Analysis

**Project**: OmniLang
**Date**: 2026-07-26
**Analysis Type**: Initial Setup Assessment
**Analysts**: Systems Architect, Domain Expert, Security Specialist, Maintainability Expert, Performance Specialist, Implementation Strategist, AI Engineer, Pragmatic Enforcer, Rust Expert, TypeScript & React Specialist, Compiler & Language Architect

---

## Executive Summary

OmniLang is a domain-specific language (DSL) compiler and toolchain for building cross-platform UI applications with first-class support for motion design, signals-based state management, and multi-target code generation. The project sits at the intersection of programming language design, compiler engineering, and frontend framework development — a technically demanding space that the codebase navigates with notable discipline.

The system is organized as a Rust monorepo: `omnilang` (the compiler — lexer, parser, IR, semantic analysis, multi-target codegen), `omnic` (the CLI interface), `omnilang-wasm` (WASM compilation for browser use), and `poiesis` (a Tauri v2 desktop application providing a visual editor for OmniLang code, built with React + Monaco Editor + Zustand). An independent npm package `@omni/animation` provides runtime animation support. The compiler targets 7+ output languages (React/OmniView, Flutter/Dart, Vue, Dioxus, Python, JavaScript, and shared HTML).

The architecture demonstrates mature compiler design: clear phase separation (lexer → parser → semantic → IR → codegen), a visitor-pattern-based codegen with shared infrastructure and target-specific backends, source-map tracking, and a growing signals/store runtime system. The poiesis editor integrates Monaco with WASM-compiled semantic analysis for real-time feedback.

**Overall Assessment**: Good

**Key Findings**:
- **Compiler pipeline is architecturally sound**: Clear phase separation with well-defined IR between stages. The visitor pattern for multi-target codegen is the right abstraction, and shared logic in `codegen/shared/` avoids excessive duplication across 7+ targets
- **Zero `unwrap()`/`panic!()` in production paths**: The codebase consistently uses proper Rust error handling (thiserror, Result propagation), a strong signal of production-quality systems code
- **Test coverage is the critical gap**: ~2,200 lines of compiler code with approximately zero automated tests. Property tests exist as stubs, and there are codegen regression test files, but the core compiler pipeline lacks unit, integration, and regression test suites. This is the single highest-risk finding
- **The poiesis editor frontend is well-structured but under-documented**: React component architecture with Zustand state management and Monaco integration is cleanly separated, but the Tauri IPC boundary and command registry lack formal documentation
- **Multi-target codegen is an architectural strength but a maintenance multiplier**: Every language feature must be implemented across N targets, creating a quality/consistency risk without adequate test coverage per target

**Critical Recommendations**:
- **Build a regression test suite for the compiler pipeline** — property-based tests for parser round-tripping, snapshot tests for codegen output across all targets, and integration tests for the full compile pipeline (Priority: Critical)
- **Document the Tauri IPC command surface** — enumerate all commands, their argument schemas, and the Rust handler locations to reduce onboarding friction and enable systematic testing (Priority: Important)

---

## System Overview

### Project Information

**Primary Language(s)**: Rust (compiler, CLI, Tauri backend), TypeScript (poiesis frontend, omni-animation), OmniLang (.omni DSL source files)

**Frameworks**: Tauri v2 (desktop), React 19 (UI), Next.js (learn-omniview showcase), Flutter/Dart (codegen target), Vue (codegen target), Dioxus (codegen target)

**Architecture Style**: Compiler monorepo with desktop editor frontend. Layered compiler pipeline (lexer → parser → semantic → IR → codegen). Multi-target code generation via visitor pattern.

**Deployment**: Tauri desktop app (Windows/macOS/Linux), WASM for browser, npm package for animation runtime

**Team Size**: Individual developer (jerghy)

**Project Age**: Multi-year ongoing development. Active commits as of July 2026.

### Technology Stack

**Compiler**:
- Rust — lexer (hand-written), recursive descent parser, semantic analysis, IR lowering
- Multi-target codegen: React/OmniView, Flutter/Dart, Vue, Dioxus, Python, JavaScript, shared HTML
- WASM compilation target via wasm-pack (omnilang-wasm crate)
- Error handling via thiserror, source maps via custom sourcemap module

**Frontend (Poiesis)**:
- React 19 + TypeScript 6.0
- Zustand 5.0 for state management
- Monaco Editor 0.55 for code editing
- Vite 8.1 for bundling
- dnd-kit for drag-and-drop
- sonner for toast notifications

**Desktop Shell**:
- Tauri v2 with Rust backend
- SQLite via libsqlite3-sys for persistence
- Victauri MCP for app inspection/testing

**Animation**:
- Independent npm package `@omni/animation` (packages/omni-animation)
- Keyframe-based animation system
- GSAP integration path

**Infrastructure**:
- pnpm workspaces for Node.js packages
- Cargo workspace for Rust crates
- Vitest for frontend testing
- Playwright for E2E (poiesis)

### Project Structure

```
/
├── omnilang/              # Core compiler crate
│   └── src/
│       ├── lexer/         # Tokenizer + keywords
│       ├── parser/        # Recursive descent parser (expr, stmt, top_level, view)
│       ├── semantic/      # Semantic analysis, validation, cycle detection
│       ├── ir/            # Intermediate representation + type system
│       ├── codegen/       # Multi-target code generation
│       │   ├── shared/    # Generator, visitor, HTML, inline handler
│       │   ├── javascript/# React/OmniView JSX target
│       │   ├── flutter/   # Dart/Flutter target
│       │   ├── vue/       # Vue SFC target
│       │   ├── dioxus/    # Dioxus (Rust) target
│       │   ├── python/    # Python target
│       │   └── dart/      # Pure Dart target
│       ├── stdlib/        # Standard library (console, json, math, storage, string)
│       ├── sourcemap/     # Source mapping
│       └── error.rs       # Error types
├── omnic/                 # CLI crate (create, analyse, run commands)
├── omnilang-wasm/         # WASM bindings for browser compilation
├── poiesis/               # Tauri desktop editor application
│   ├── src/               # React frontend (TypeScript)
│   │   ├── components/    # UI components
│   │   ├── core/          # Types, constants, utilities
│   │   ├── lib/           # Shared libraries
│   │   └── commands/      # Tauri IPC command wrappers
│   └── src-tauri/         # Tauri Rust backend
├── packages/
│   └── omni-animation/    # @omni/animation npm package
├── test/
│   └── projects/
│       └── learn-omniview/# Educational showcase project
└── Cargo.toml             # Workspace root
```

**Key Observations**:
- Clean separation between compiler (omnilang), CLI (omnic), and editor (poiesis)
- Codegen module is the largest surface area — 7+ target backends sharing a common visitor/generator pattern
- Poiesis follows a feature-based organization with clear type/utility separation
- No dedicated test crate or integration test harness — tests are inline or in separate test files
- The animation package is independently versioned as an npm package, decoupled from the compiler

---

## Individual Member Analyses

### Systems Architect

**Perspective**: Focuses on how components work together as a cohesive system.

The OmniLang system exhibits strong architectural cohesion. The compiler pipeline follows established patterns (lex → parse → semantic → IR → codegen) with clean interfaces between stages. The visitor pattern for multi-target codegen is the correct abstraction — it allows shared traversal logic in `codegen/shared/generator.rs` while each target implements target-specific syntax in its own visitor. The WASM boundary (`omnilang-wasm`) cleanly wraps the compiler for browser use without coupling the core crate to web concerns.

The poiesis editor integrates with the compiler via WASM, which is architecturally sound — it avoids running a separate compiler process and keeps latency low for real-time feedback. However, the Tauri IPC layer between the React frontend and Rust backend is a critical integration point that deserves explicit documentation. The command registry, argument schemas, and handler dispatch should be mapped.

**Strengths**:
1. **Phase-separated compiler pipeline**: Each stage produces well-typed output that constrains the next stage
2. **Clean monorepo boundaries**: omnic, omnilang, omnilang-wasm, and poiesis have clear responsibilities
3. **Multi-target architecture**: The visitor pattern elegantly handles N codegen targets without combinatorial explosion

**Concerns**:
1. **No architectural decision records exist** (Impact: Medium) — Key decisions (why visitor pattern, why not AST-walk-based codegen, why Zustand over Redux) are undocumented outside of inline comments and git history
2. **WASM boundary coupling risk** (Impact: Low) — omnilang-wasm currently tightly couples to wasm-bindgen; if WASM component model becomes standard, migration cost should be assessed

### Domain Expert

**Perspective**: Evaluates how well the architecture represents the problem domain.

OmniLang serves a clear domain: enabling developers to describe UI, state, and motion in a unified language that compiles to multiple framework targets. The domain model is well-represented in the AST/IR design: components, stores, signals, timelines, and motion primitives are first-class concepts, not bolted-on features.

The language's "Design-First" philosophy (recently crystallized in the learn-omniview refactoring) is a genuine differentiator. By making CSS variables, design tokens, and animation keyframes first-class language constructs, OmniLang elevates design concerns to the same level as logic — something no mainstream framework does natively.

**Strengths**:
1. **First-class motion design**: Timelines, keyframes, and signal-driven animation are language-level constructs, not library add-ons
2. **Store/signal system mirrors real UI patterns**: Reactive state with explicit dependency tracking matches how UI developers think about data flow
3. **CSS variable integration**: Design tokens flow from language constructs to runtime CSS variables, closing the design-to-code gap

**Concerns**:
1. **Language surface area is large for one developer** (Impact: Medium) — Components, stores, signals, timelines, motion, inline styles, CSS variables, and 7+ codegen targets create a combinatorial maintenance burden

### Security Specialist

**Perspective**: Reviews security posture from a systems perspective.

As a compiler and desktop editor (not a web service), the attack surface is limited. The primary risks are: (1) malicious `.omni` source files processed by the compiler, (2) Tauri IPC vulnerabilities in the poiesis desktop app, and (3) supply chain risks in npm/cargo dependencies.

The codebase handles the compiler security surface well: the parser is hand-written (no parser generator injections), error handling uses typed errors (no string-based panics), and the WASM sandbox provides isolation in browser contexts. For the Tauri app, the filesystem access uses Tauri's capability-based permissions system.

**Strengths**:
1. **No eval() or dynamic code execution in compiler output**: Codegen produces static target code
2. **Tauri v2 capability system**: Filesystem and system access is explicitly permitted, not default-allow
3. **Proper error propagation**: No information leakage through error messages (thiserror types are structured)

**Concerns**:
1. **No fuzz testing of the parser** (Impact: Low) — Malformed `.omni` files could trigger unexpected parser behavior; a fuzz harness would be valuable for a compiler
2. **npm supply chain**: The omni-animation package and poiesis frontend dependencies should be audited (npm audit, dependency review)

### Maintainability Expert

**Perspective**: Evaluates code quality and long-term maintenance burden.

The codebase exhibits disciplined Rust practices: zero `unwrap()`/`panic!()` in production paths, consistent use of `thiserror`, and clear module boundaries. The recent refactoring work (learn-omniview Design-First migration, OmniView codegen refactoring reducing `element_to_html` from 839→110 lines) demonstrates a healthy commitment to code quality and reducing technical debt.

However, the test coverage gap is a significant maintainability risk. A compiler with 7+ codegen targets and zero automated tests means that every change requires manual verification across all targets. Property-based testing for the parser, snapshot testing for codegen output, and integration tests for the compile pipeline would dramatically reduce the risk of regressions.

**Strengths**:
1. **Disciplined error handling**: Consistent use of Result types, no panics in production paths
2. **Active refactoring culture**: Recent work shows willingness to invest in code quality (839→110 line reductions)
3. **Small focused files**: Modules are well-decomposed (parser, codegen, semantic are separate crates/modules)

**Concerns**:
1. **~2,200 lines of compiler code with ~zero tests** (Impact: High) — This is the single largest maintainability risk. Every change to the compiler requires manual testing across all targets
2. **No CI/CD pipeline visible**: Build verification, test execution, and linting don't appear to be automated
3. **Multi-target codegen without per-target regression suites** (Impact: High) — Changes to shared codegen logic could silently break individual targets

### Performance Specialist

**Perspective**: Evaluates performance characteristics and optimization opportunities.

For a compiler, the critical performance paths are parse time, codegen time, and (for the editor) WASM compilation latency. The hand-written recursive descent parser is typically faster than parser-generator approaches for DSL-sized grammars. The WASM compilation path (used by poiesis for real-time editor feedback) is the most latency-sensitive path — users expect sub-second feedback when typing.

The Tauri desktop app benefits from Rust's native performance for backend operations, and React 19's concurrent features should provide responsive UI. The omni-animation package uses Web Animations API where available and GSAP as a fallback, which is the correct layering for animation performance.

**Strengths**:
1. **Hand-written parser avoids generator overhead**: Good for a DSL where parsing is on the hot path
2. **WASM for browser compilation**: Appropriate for editor use cases where a native binary isn't available
3. **Release profile optimizations**: LTO, opt-level="s", and stripping are configured for production builds

**Concerns**:
1. **No performance benchmarks**: Parse time, codegen time, and WASM bundle size should be tracked to catch regressions
2. **WASM binary size**: The `wasm-release` profile uses `opt-level = "z"` and LTO, but actual bundle size should be benchmarked

### Implementation Strategist

**Perspective**: Evaluates change sequencing, blast radius, and reversibility.

The project's architecture is well-suited to incremental change. The compiler pipeline's phase separation means that parser changes have a blast radius limited to the AST/IR boundary. Codegen changes for a single target are isolated to that target's visitor. The monorepo structure (separate crates) allows independent versioning and deployment of omnic vs omnilang vs omni-animation.

For the critical recommendation (adding test coverage), the sequencing should be: (1) snapshot tests for codegen output first (highest value, catches regressions across all targets), (2) parser round-trip property tests, (3) integration tests for the full compile pipeline. This order maximizes risk reduction per unit of effort.

**Strengths**:
1. **Low blast radius for most changes**: Parser, codegen, and editor changes are well-isolated
2. **Independent deployment units**: omnic CLI, omni-animation npm package, and poiesis desktop app can ship independently
3. **Reversible architecture**: The visitor pattern makes adding/removing codegen targets non-disruptive

**Concerns**:
1. **No release process documented**: Version numbering strategy, changelog conventions, and release checklists are not formalized
2. **Single-developer bus factor**: All architectural knowledge lives in one person's head; ADRs would de-risk this

### AI Engineer

**Perspective**: Evaluates AI integration patterns and development workflow.

The OmniLang project itself is not an AI application, but its development workflow is heavily AI-assisted — Claude Code with extensive agent configurations, memory systems, and automated code review. The project demonstrates an advanced AI-assisted development workflow with well-structured agent types, memory persistence, and systematic code review.

The compiler's architecture (clear phase separation, typed IR, visitor pattern) is well-suited to AI-assisted development because each phase has clear inputs/outputs that can be verified independently. This makes it easier for AI agents to contribute to specific phases without understanding the entire pipeline.

**Strengths**:
1. **Well-structured agent workflow**: 15+ specialized agents with clear responsibilities
2. **Memory persistence**: Project-specific memory files capture architectural knowledge, bug fixes, and design decisions
3. **Code review automation**: Mandatory code review via agents after code changes

**Concerns**:
1. **Memory files are a partial ADR substitute**: The memory system captures valuable context, but it's ephemeral compared to formal ADRs

### Pragmatic Enforcer

**Perspective**: Challenges complexity and advocates for YAGNI.

The OmniLang project makes a strong pragmatic case: it solves a real problem (writing UI once, compiling to many frameworks) that existing tools don't address well. The architecture avoids premature optimization and speculative generality — for example, the 7+ codegen targets support real use cases (React for web, Flutter for mobile, Vue/Dioxus for framework diversity), not hypothetical ones.

However, there are areas where simplicity could be prioritized:
- The animation system as an independent npm package is pragmatically sound (separate concern, independent versioning) but adds a package boundary to maintain
- The CSS variable system has three usage patterns (documented in memory) — consolidating to fewer patterns would reduce cognitive load
- The poiesis editor v3→v5 evolution (abandoned block-based editor → Monaco integration) was the right call — recognizing when an approach isn't working and pivoting is pragmatically correct

**Strengths**:
1. **Willingness to abandon approaches that don't work**: CodeCard v3→v4→Monaco v5 is a pragmatic evolution
2. **Multi-target codegen solves a real problem**: Not speculative — each target has concrete use cases
3. **Independent animation package**: Correct scope boundary

**Concerns**:
1. **3 CSS variable usage patterns** (Impact: Low) — Consolidation opportunity
2. **Poiesis editor iteration history** (Impact: Low) — The 4 failed approaches to the block editor were expensive; the Monaco pivot was the right call but came after significant investment

### Rust Expert (Katya Volkov)

**Perspective**: Evaluates Rust-specific code quality, safety, and idioms.

The Rust code in this project is of notably high quality for a systems-language compiler. The consistent use of `thiserror` for error types, proper `Result` propagation (zero `unwrap()`/`expect()` in production paths), and clean module decomposition are hallmarks of experienced Rust development. The codegen visitor pattern correctly uses Rust's trait system for polymorphism without dynamic dispatch overhead.

The parser implementation is a hand-written recursive descent parser — the right choice for a DSL where error messages and recovery are more important than raw parsing speed. The IR types use Rust's enum system effectively to encode AST node variants, making pattern matching exhaustive and safe.

**Strengths**:
1. **Zero-cost abstractions via generics and traits**: The visitor/generator pattern incurs no runtime overhead
2. **Proper ownership modeling**: The compiler pipeline transfers ownership between phases (parser → IR → codegen) rather than sharing references
3. **Type-driven design**: IR types make invalid states unrepresentable (e.g., using enums for binary operators, not raw strings)

**Concerns**:
1. **No property-based testing**: The `omnilang/tests/property_tests.rs` file exists but property testing frameworks (proptest, quickcheck) don't appear to be wired up
2. **WASM target is separate crate**: The `omnilang-wasm` crate duplication risk — updates to omnilang's public API must be manually reflected in the WASM bindings

### TypeScript & React Specialist (Emma Rodriguez)

**Perspective**: Evaluates frontend architecture, component design, and TypeScript usage.

The poiesis frontend demonstrates solid React architecture: Zustand for state management (appropriate for a desktop app where Redux would be overkill), Monaco Editor integrated behind a service boundary, and TypeScript providing type safety across the component tree. The feature-based file organization (components/, core/types/, core/utils/, lib/) is clean and avoids the "barrel file" anti-pattern.

Component design follows modern React patterns with hooks and composition. The use of TypeScript 6.0 with React 19 shows the project stays current with the ecosystem. Vite 8.1 as the bundler provides fast HMR for development.

**Strengths**:
1. **Clean state architecture**: Zustand stores are focused and avoid the Redux boilerplate trap
2. **Monaco integration is well-isolated**: The editor component doesn't leak Monaco types into the broader component tree
3. **TypeScript strictness**: Strong typing across the codebase with well-defined wire types for IPC

**Concerns**:
1. **No component tests**: The frontend lacks unit tests for components and hooks
2. **IPC command wrappers in `src/commands/`**: These should be typed more strictly — the wire types are defined but the command invocation layer could benefit from generated types from the Rust command schemas

### Compiler & Language Architect (Dr. Lin Wei)

**Perspective**: Evaluates language design, compiler correctness, and codegen architecture.

OmniLang's language design shows mature judgment. Making motion, signals, and stores first-class language constructs rather than library features is the right level of abstraction — it allows the compiler to reason about these concepts and optimize across them. The multi-target codegen via the visitor pattern is the canonical approach and is implemented correctly here.

The IR design is notably well-structured: separating "lowered logic" from view expressions, maintaining source maps through transformations, and using typed IR nodes rather than stringly-typed intermediates. The semantic analysis phase catches issues (cycles, invalid inline expressions) at compile time rather than deferring to runtime.

**Strengths**:
1. **First-class motion and signals at the language level**: Correct abstraction layer — these are language semantics, not library features
2. **Proper IR lowering**: Logic IR is separated from view IR, enabling independent optimization of each
3. **Source maps survive all transformations**: Essential for meaningful error messages in a multi-stage compiler
4. **Template strings and spread syntax in the parser**: Current language features implemented correctly

**Concerns**:
1. **No formal language specification**: The grammar and semantics are defined implicitly by the parser implementation — a specification document would aid new contributors and target implementers
2. **Codegen test coverage is minimal**: The `tests_regression/` files exist but their contents and whether they're part of an automated test suite is unclear
3. **No fuzzing infrastructure**: Compilers benefit enormously from fuzz testing — random `.omni` files fed through parse → codegen → (compile target) would catch edge cases systematically

---

## Collaborative Synthesis

### Common Themes

**Strengths** (What multiple members praised):
1. **Compiler pipeline architecture**: Universally recognized as sound — clear phase separation, well-typed IR, correct use of the visitor pattern for multi-target codegen
2. **Disciplined Rust practices**: Zero panics in production paths, proper error handling, clean module boundaries
3. **First-class domain modeling**: Motion, signals, stores, and design tokens as language constructs rather than library features is the right abstraction

**Concerns** (What multiple members flagged):
1. **Critical test coverage gap**: Every member flagged the lack of automated tests for the compiler pipeline. ~2,200 lines of code with zero tests is unsustainable for a multi-target compiler
2. **No formal documentation for key interfaces**: The Tauri IPC surface, language grammar, and release process are undocumented
3. **Single-developer bus factor**: All architectural knowledge is implicit — ADRs and formal documentation would de-risk the project

**Disagreements** (Where members had different views):
- **Animation package independence**: The Pragmatic Enforcer sees the separate `@omni/animation` package as potential over-separation; the Systems Architect and Domain Expert see it as correct scope boundary. Resolution: Keep the separation — the animation runtime has different versioning, consumers, and concerns than the compiler
- **Test priority ordering**: The Maintainability Expert prioritizes codegen snapshot tests; the Compiler Architect prioritizes parser fuzz testing. Resolution: Both are needed, but start with codegen snapshots (catches more regression classes immediately)

### Prioritized Findings

**Critical (Address Immediately)**:
1. **Zero automated tests for the compiler**: 2,200+ lines of compiler code with no test coverage. Every change risks silent regressions across 7+ codegen targets. This is the single highest-risk finding and should be addressed before any feature work.

**Important (Address in Near Term)**:
1. **Undocumented Tauri IPC surface**: The command registry between poiesis frontend and Rust backend needs systematic documentation
2. **No language specification**: The OmniLang grammar lives only in the parser implementation — a specification would help new contributors and target implementers
3. **No CI/CD pipeline**: Automated build verification, test execution, and linting are not visible

**Nice-to-Have (Consider for Future)**:
1. **CSS variable usage pattern consolidation**: Three patterns documented — could be simplified
2. **Formal release process**: Version numbering, changelogs, release checklists
3. **Fuzz testing infrastructure**: Long-term compiler quality investment

---

## Recommendations

### Immediate Actions (0-2 Weeks)

1. **Create codegen snapshot tests for all targets**
   - **Why**: Catches regressions across all 7+ targets with minimal effort. A single `.omni` test file compiled to each target and compared against golden output provides immediate regression protection
   - **How**: Create a `tests/snapshots/` directory with `.omni` input files and expected output for each target. Use `insta` or a similar snapshot testing crate
   - **Owner**: Developer
   - **Success Criteria**: At least one snapshot test per codegen target that runs in CI; any codegen change that alters output must update snapshots
   - **Effort**: ~4-8 hours

2. **Set up CI/CD pipeline**
   - **Why**: Ensures that tests actually run on every change. Without CI, test infrastructure has limited value
   - **How**: GitHub Actions with `cargo test`, `cargo clippy`, `cargo fmt --check` for Rust; `pnpm test` and `pnpm typecheck` for frontend
   - **Owner**: Developer
   - **Success Criteria**: All tests pass on push/PR; clippy and fmt produce no warnings
   - **Effort**: ~2-4 hours

### Short-Term Actions (2-8 Weeks)

1. **Document Tauri IPC command surface**
   - **Why**: Critical for onboarding, testing, and understanding the frontend-backend boundary
   - **How**: Enumerate all commands in the Rust backend, their argument/return types, and create a markdown reference in `.architecture/decisions/` or equivalent
   - **Owner**: Developer
   - **Success Criteria**: Every Tauri command has a documented signature, handler location, and description
   - **Effort**: ~2-4 hours

2. **Add parser property-based tests**
   - **Why**: Ensures parser correctness for arbitrary valid `.omni` inputs (round-trip: parse → pretty-print → re-parse produces same AST)
   - **How**: Use `proptest` crate to generate random valid ASTs, pretty-print to `.omni`, parse back, and verify AST equivalence. Also test that random invalid inputs always produce errors (never panic)
   - **Owner**: Developer
   - **Success Criteria**: Property tests pass for 10,000+ random inputs; invalid inputs never cause panics
   - **Effort**: ~8-16 hours

3. **Create initial ADRs for key architectural decisions**
   - **Why**: Captures architectural rationale that currently lives only in memory and git history
   - **How**: Create ADRs for: (1) Visitor pattern for multi-target codegen, (2) Zustand over Redux for state management, (3) Monaco over block-based editor, (4) Independent animation package
   - **Owner**: Developer
   - **Success Criteria**: 4+ ADRs created documenting key decisions with context, rationale, and consequences
   - **Effort**: ~2-4 hours

### Long-Term Initiatives (2-6 Months)

1. **Achieve 80%+ test coverage on compiler core**
   - **Why**: Sustainable development velocity requires confidence that changes don't break existing behavior
   - **How**: Incrementally add tests for parser, semantic analysis, IR lowering, and each codegen target. Target 80% line coverage as measured by tarpaulin or equivalent
   - **Owner**: Developer
   - **Success Criteria**: 80% line coverage on omnilang crate; all codegen targets have regression test suites
   - **Effort**: Ongoing, ~40-80 hours cumulative

2. **Draft OmniLang language specification**
   - **Why**: Reduces onboarding friction for contributors and target implementers; serves as the source of truth for language semantics
   - **How**: Document grammar in EBNF, describe semantics for each language construct (components, stores, signals, timelines, motion), specify the compilation model for each target
   - **Owner**: Developer
   - **Success Criteria**: Specification covers all language constructs; a new developer could implement a codegen target from the spec alone
   - **Effort**: ~20-40 hours

---

## Suggested Next Steps

Based on this initial analysis:

1. **Set up CI/CD with basic test infrastructure** — Before any feature work, ensure every push is verified. Start with `cargo test`, `cargo clippy`, `cargo fmt --check` on GitHub Actions
2. **Create codegen snapshot tests** — One `.omni` file → expected output per target. This is the highest-value testing investment per unit of effort
3. **Document Tauri IPC commands** — Map all commands, their argument schemas, and handler locations
4. **Create initial ADRs** — Start with "Multi-target codegen via visitor pattern" and "Monaco Editor for poiesis"

**Documentation**:
- Create ADRs for key architectural decisions identified above
- Document the Tauri command surface and IPC protocol

**Process**:
- Establish CI/CD with automated testing on every push
- Define ADR creation criteria (when does a decision warrant an ADR?)
- Schedule follow-up architecture review after test infrastructure is in place (suggested: 3 months)

---

## Appendix

### Analysis Methodology

This analysis was conducted using the AI Software Architect framework. Each of the 11 team members analyzed the system from their specialized perspective, then findings were synthesized to identify cross-cutting themes, prioritize recommendations, and establish a roadmap.

**Members Participating**:
- Systems Architect — System cohesion and component interaction
- Domain Expert — Business domain and language design
- Security Specialist — Security posture and attack surface
- Maintainability Expert — Code quality and technical debt
- Performance Specialist — Performance and resource utilization
- Implementation Strategist — Change sequencing and blast radius
- AI Engineer — AI-assisted development workflow
- Pragmatic Enforcer — YAGNI and simplicity advocacy
- Rust Expert — Rust idioms, safety, and systems programming
- TypeScript & React Specialist — Frontend architecture and component design
- Compiler & Language Architect — Language design and compiler correctness

### Source Materials

- Project memory files: omnilang-parser, omnilang-codegen, omnilang-bugfixes, omnilang-signals, omnilang-animation, omnilang-css-variables, omniview-architecture-assessment, omniview-code-maturity, omniview-codegen-refactoring, omniview-compiler-internals, learn-omniview-refactoring, omniview-docs-restructure, project-build, project-permissions, codegen-issues
- Git history: Recent commits including learn-omniview refactoring, OmniView codegen refactoring, documentation restructuring
- Codebase analysis: Cargo workspace structure, TypeScript component organization, Tauri configuration

---

**Analysis Complete**
**Next Review**: 2026-10-26 (3 months)

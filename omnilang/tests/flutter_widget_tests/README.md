# Flutter widget-test harness — 生成代码运行时验证（Phase 3.2）

首个对「生成代码」做运行时执行级验证的测试框架（flutter 长期项 #2）：
`tester.pumpWidget` 冒烟测试在真实 Riverpod ProviderScope 中挂载 codegen 产出的
Flutter 组件，验证 @if / @for / store 读写的执行行为——不是字符串快照，而是
widget 树级行为断言。

## 布局

```
omnilang/tests/flutter_widget_tests/
├── README.md              # 本文档
├── harness.rs             # Rust 集成测试入口（[[test]] flutter_widget_tests）
└── smoke/                 # 冒烟测试源（Dart，单一事实源）
    ├── app_mount_test.dart     # app 挂载（ProviderScope 包装）
    ├── if_toggle_test.dart     # @if 条件渲染切换（Modal）
    ├── for_render_test.dart    # @for 列表渲染 + add 重渲染（ForDirectiveKeyDemo）
    └── store_add_test.dart     # TodoStore.add() → @for 重渲染（容器直驱；bind:value 路径延迟至 3.3）
```

物化的 Flutter 输出项目（gitignored 构建产物）：
`test/projects/learn-omniview/build/flutter-test-app/`

- `lib/` — 每次由 harness 用**当前** codegen 全量重新生成（先清空）
- `pubspec.yaml` — harness 固定模板写入（flutter_riverpod + http/intl/shared_preferences + flutter_test）
- `test/` — smoke/ 的物化副本（按 brief 提交为快照，供审查/CI 直接查看；smoke/ 才是事实源，重建即同步）

## 运行

```bash
# 默认（codegen 硬门禁 + flutter analyze 门禁；SDK 缺失时 analyze 跳过并提示）
cargo test -p omnilang --features codegen-flutter --test flutter_widget_tests

# 全量冒烟（含 flutter test；flutter test 约 30-60s 墙钟，故 opt-in）
OMNILANG_FLUTTER_TEST=1 cargo test -p omnilang --features codegen-flutter --test flutter_widget_tests

# 或直接进入输出项目运行（无需 cargo）
cd test/projects/learn-omniview/build/flutter-test-app && flutter pub get && flutter test
```

harness 断言链：① learn-omniview → Flutter codegen 零错误（硬门禁，无 SDK 也执行）
② `flutter analyze` 全项目 + 冒烟测试零 issue（SDK 存在时）③ `flutter test`
冒烟全绿（opt-in）。入口组件名对齐：生成的 app shell 类为 `OmniApp`（main.dart），
非 demo 组件名。

## 冒烟集状态

| 冒烟项 | 文件 | 状态 |
|--------|------|------|
| app 挂载（ProviderScope 包装） | smoke/app_mount_test.dart | ✅ 本任务 |
| @if 切换（Modal 开/关子树） | smoke/if_toggle_test.dart | ✅ 本任务 |
| @for 渲染（种子项 + add 重渲染） | smoke/for_render_test.dart | ✅ 本任务 |
| TodoStore.add() → @for 重渲染 | smoke/store_add_test.dart | ✅ 本任务（容器直驱） |
| bind:value 输入回写 | —（store_add_test.dart 内 DEFERRED 注释） | ⏳ 延迟至 3.3：生成 AddTodo 的 TextField 无 controller/onChanged 接线，enterText 路径当前会坏；3.3 修复后补 enterText 断言 |
| motion / ref / [scroll] | — | 后续扩展位（框架就绪，测试随功能任务补） |

## CI 接线候选（`flutter-widget-tests` job）

本任务先本地；CI 接线二选一（建议 A，零 CI 改动）：

- **A（推荐）**：现有 `codegen-validate` job 已装 flutter-action，把
  `OMNILANG_FLUTTER_TEST=1` 注入其 `cargo test -p omnilang --features codegen-flutter`
  即得全量 widget test（analyze 门禁无需改动即已生效）。
- **B**：独立 job：
  ```yaml
  - uses: subosito/flutter-action@v2
  - run: OMNILANG_FLUTTER_TEST=1 cargo test -p omnilang --features codegen-flutter --test flutter_widget_tests
  ```
  独立 job 的好处是失败归属清晰；代价是重复 setup + 墙钟。

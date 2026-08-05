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
    ├── store_add_test.dart     # TodoStore.add() → @for 重渲染（容器直驱；bind:value 路径延迟至 3.3）
    ├── form_inputs_test.dart   # FormPage textarea→TextField(maxLines:4) 可见可输入 + disabled→enabled:false 不可编辑（3.5）
    └── page_switch_test.dart   # 标签页切换保留表单输入（3.9：PageSwitcherDemo 顶层互斥 @if → IndexedStack）
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
| @for 重排 key 稳定（Move Last to Front → ValueKey 复用，Element 身份保持） | smoke/for_render_test.dart（3b） | ✅ 7.6 |
| TodoStore.add() → @for 重渲染 | smoke/store_add_test.dart | ✅ 本任务（容器直驱） |
| textarea/disabled 映射（FormPage message 可见可输入；disabled 不可编辑） | smoke/form_inputs_test.dart | ✅ 3.5 |
| bind:value 输入回写（enterText → onChanged 信号写入 → 添加按钮动作） | smoke/store_add_test.dart（enterText 用例） | ✅ 3.3（原 DEFERRED 标记已落地） |
| 标签页切换保留表单输入（PageSwitcherDemo 顶层互斥 @if → IndexedStack） | smoke/page_switch_test.dart | ✅ 3.9 |
| motion @enter 挂载即播（AnimPropertiesDemo：Opacity 0.3→1.0 起动 + 完成） | smoke/motion_enter_test.dart | ✅ 7.6 |
| motion @in-view / @focus 触发器播放 | smoke/inview_focus_trigger_test.dart | ✅ 3.4 |
| [scroll] ListView + store 读取（Riverpod 3.0 scope） | smoke/scroll_store_test.dart | ✅ 3.7 |
| theme 卡片 → ThemeData 切换（4 主题可达） | smoke/theme_switcher_test.dart | ✅ 3.6 |

## CI 接线（Task 7.6 已落地）

`codegen-validate` job（.github/workflows/ci.yml）的「Test omnilang with
codegen-flutter」步骤已注入 `OMNILANG_FLUTTER_TEST=1` —— `flutter test`
冒烟套件随该 job 全量执行（选项 A；analyze 门禁无需改动即已生效）。
React 侧对应断言集在 `test/projects/learn-omniview/build/react-test-app/tests/`
（behavioral-parity.spec.ts + theme-wiring.spec.ts + omniview-react.spec.ts），
随 e2e job 的 Playwright 套件执行——5 行为场景双目标均入 CI。

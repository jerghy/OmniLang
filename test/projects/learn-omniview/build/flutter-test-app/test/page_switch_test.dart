// 批次 2（导航路由）— 路由形态断言：HomePage/AboutPage 由 @route 声明、
// navigate("/about") 经 GoRouter context.push 切换。
//
// 诚实降级注记（spec §6 边界 7）：v0 路由形态为 push/pop 栈切换——上一页
// 被覆盖而非保挂载，且"返回"按钮走 push("/home") 新开实例，typed draft
// 不保留（StatefulShellRoute.indexedStack 状态保持留待后续）。
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'package:flutter_test_app/main.dart';
import 'package:flutter_test_app/router.dart';

void main() {
  testWidgets('routed HomePage/AboutPage switch via GoRouter push', (
    tester,
  ) async {
    // 视图高度须容纳 Main 根组件（Overview 首页内容较多）——与
    // app_mount_test 的 6000 高视口一致，避免 RenderFlex overflow。
    tester.view.physicalSize = const Size(1440, 6000);
    tester.view.devicePixelRatio = 1.0;
    addTearDown(tester.view.reset);

    await tester.pumpWidget(const ProviderScope(child: OmniApp()));
    await tester.pumpAndSettle();

    // initialLocation '/' — Main 根组件（9 标签页 hub）
    expect(
      find.textContaining('OmniView 能力展示'),
      findsOneWidget,
      reason: 'initialLocation / 必须渲染 Main 根组件',
    );

    // 直达路由 /home（PageSwitcherDemo.HomePage @route）
    router.go('/home');
    await tester.pumpAndSettle();
    expect(
      find.widgetWithText(TextField, 'Home draft'),
      findsOneWidget,
      reason: '/home 路由必须渲染 HomePage（含 Home draft 输入框）',
    );

    // 输入后 navigate("/about")
    await tester.enterText(
      find.widgetWithText(TextField, 'Home draft'),
      'draft note',
    );
    await tester.pump();
    await tester.tap(find.text('Go to About').hitTestable());
    await tester.pumpAndSettle();
    expect(
      find.text('Back to Home').hitTestable(),
      findsOneWidget,
      reason: 'navigate("/about") 必须渲染 AboutPage',
    );

    // 返回 /home（按钮 push("/home") 新开实例）
    await tester.tap(find.text('Back to Home').hitTestable());
    await tester.pumpAndSettle();
    expect(
      find.widgetWithText(TextField, 'Home draft'),
      findsOneWidget,
      reason: '返回后必须再次渲染 HomePage',
    );
    // §6 边界 7：路由形态 push 栈切换 + 返回按钮新开实例——draft 不保留
    expect(
      find.text('draft note'),
      findsNothing,
      reason: '路由形态下页面不保挂载，typed draft 不保留（spec §6 边界 7 '
          '诚实降级注记）',
    );
    expect(tester.takeException(), isNull);
  });
}

import { test, expect } from '@playwright/test';

// ResponsiveDemo（批次 3 响应式）浏览器行为断言（spec §7）：
//   1. 视口 ≥768（1024）：DesktopLayout 分支可见 + 移动分支隐藏
//   2. 视口 <768（500）：MobileLayout 分支可见 + 桌面分支隐藏
//   3. 边界精度：767 走移动、768 走桌面（min-width 语义，无 off-by-one）
//   4. @media 块规则在 web 生效（≥768 时 .card 内边距变化）
//   5. CLI W-301 stderr 断言——主责任在 omnic 单测（cmd_build_surfaces_w301_
//      without_failing，构建不失败契约）；浏览器侧无 CLI 调用先例，以单测为准。
//
// ResponsiveDemo 位于 UiComponents 页（TABS[1]='🎨 UI'）。分支文本：
//   - 桌面分支（≥768px 可见）
//   - 移动分支（宽度小于 768px 可见）

const BASE = 'http://localhost:5173';

async function gotoUiPage(page: import('@playwright/test').Page) {
  await page.goto(BASE, { timeout: 8000 });
  await page.waitForSelector('button', { timeout: 8000 });
  await page.locator('button:has-text("🎨 UI")').first().click();
  await page.waitForTimeout(500);
}

test.describe('ResponsiveDemo（批次 3）', () => {
  test('视口 ≥768：桌面分支可见 + 移动分支隐藏', async ({ page }) => {
    await gotoUiPage(page);
    await page.setViewportSize({ width: 1024, height: 800 });
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeVisible();
    await expect(page.getByText('移动分支（宽度小于 768px 可见）')).toBeHidden();
  });

  test('视口 <768：移动分支可见 + 桌面分支隐藏', async ({ page }) => {
    await gotoUiPage(page);
    await page.setViewportSize({ width: 500, height: 800 });
    await expect(page.getByText('移动分支（宽度小于 768px 可见）')).toBeVisible();
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeHidden();
  });

  test('边界精度 767/768：min-width 语义无 off-by-one', async ({ page }) => {
    await gotoUiPage(page);
    await page.setViewportSize({ width: 767, height: 800 });
    await expect(page.getByText('移动分支（宽度小于 768px 可见）')).toBeVisible();
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeHidden();
    await page.setViewportSize({ width: 768, height: 800 });
    await expect(page.getByText('桌面分支（≥768px 可见）')).toBeVisible();
    await expect(page.getByText('移动分支（宽度小于 768px 可见）')).toBeHidden();
  });

  test('@media 块规则在 web 生效（≥768 时 .card 内边距变化）', async ({ page }) => {
    await gotoUiPage(page);
    // ResponsiveDemo 的 .card 是 LearnDesign.Card + 本地 card 类——取首个
    // 含「响应式布局演示」标题的卡片容器比较内边距。
    const card = page.getByText('响应式布局演示（bp() / @media）').locator('xpath=..');
    await page.setViewportSize({ width: 500, height: 800 });
    const padSmall = await card.evaluate((el) => getComputedStyle(el).paddingTop);
    await page.setViewportSize({ width: 1024, height: 800 });
    const padLarge = await card.evaluate((el) => getComputedStyle(el).paddingTop);
    expect(padLarge).not.toBe(padSmall);
    // 明确断言 16px → 32px 语义（@media (min-width: 768px) .card 增强）
    expect(padSmall).toBe('16px');
    expect(padLarge).toBe('32px');
  });
});

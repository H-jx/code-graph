/**
 * 研发采购申请台账 E2E
 * 路径：tests/e2e/specs/routes/other-purchase-ledger.spec.ts
 *
 * 覆盖路由：/Other/purchaseLedger（name: purchaseLedger）
 * 权限 access：canCreateRootProcureRequisition
 *
 * 测试场景：
 *  - 已登录有权限用户 → 访问页面 → URL 命中、Tab 切换、表格渲染
 *  - 无权限用户 → 访问页面 → 跳转/403/菜单隐藏（按 access 实现决定）
 */

import { test, expect } from '../../fixtures/base';
import { adminUser } from '../../fixtures/seed';

test.describe('研发采购申请台账', () => {
  /**
   * 导航到台账页面并等待内容加载完成
   */
  test.beforeEach(async ({ page, msw }) => {
    void msw;
    await page.goto('/#/Other/purchaseLedger');
    // 等待 URL 确定在台账页
    await expect(page).toHaveURL(/#\/Other\/purchaseLedger/, { timeout: 15000 });
    // 等待表格加载完成（ProTable 渲染出 th）
    await page.waitForSelector('table th', { timeout: 15000 });
  });

  test('已登录有权限用户：URL 命中 /Other/purchaseLedger', async ({ page }) => {
    await expect(page).toHaveURL(/#\/Other\/purchaseLedger/);
  });

  test('已登录有权限用户：页面渲染两个主 Tab（采购申请 / 采购申请明细）', async ({
    page,
  }) => {
    // 断言默认激活「采购申请」Tab
    const requestTab = page.getByRole('tab', { name: /^采购申请$/ });
    await expect(requestTab).toBeVisible();
    await expect(requestTab).toHaveAttribute('aria-selected', 'true');

    // 断言第二个主 Tab「采购申请明细」存在
    const detailTab = page.getByRole('tab', { name: /采购申请明细/ });
    await expect(detailTab).toBeVisible();
  });

  test('已登录有权限用户：采购申请 Tab 下渲染状态子 Tab（待提交/审批中/已批准/已删除）', async ({
    page,
  }) => {
    // 断言默认「待提交」状态激活
    const pendingSubmitTab = page.getByRole('tab', { name: /待提交/ });
    await expect(pendingSubmitTab).toBeVisible();
    await expect(pendingSubmitTab).toHaveAttribute('aria-selected', 'true');

    // 断言其他状态 Tab 存在
    const pendingApprovalTab = page.getByRole('tab', { name: /审批中/ });
    await expect(pendingApprovalTab).toBeVisible();

    const approvedTab = page.getByRole('tab', { name: /已批准/ });
    await expect(approvedTab).toBeVisible();

    const deletedTab = page.getByRole('tab', { name: /已删除/ });
    await expect(deletedTab).toBeVisible();
  });

  test('已登录有权限用户：默认显示待提交状态数据，表格有申请单号列', async ({
    page,
    msw,
  }) => {
    // 断言表格有数据（默认状态 PENDING_SUBMIT）
    await expect(page.getByText('RPR-2025-001')).toBeVisible({ timeout: 10000 });

    // 断言有关键列头：申请单号
    const requestNoHeader = page.locator('table th').filter({ hasText: '申请单号' }).first();
    await expect(requestNoHeader).toBeVisible();

    // 断言 MSW 拦截到 purchaseRequests 请求
    expect(msw.getLastCall('purchaseRequests')).toBeDefined();
  });

  test('已登录有权限用户：切换到「审批中」状态 Tab，明细数据切换', async ({
    page,
    msw,
  }) => {
    // 点击「审批中」状态 Tab
    const pendingApprovalTab = page.getByRole('tab', { name: /审批中/ });
    await pendingApprovalTab.click();
    await page.waitForTimeout(500);

    // 断言「审批中」Tab 激活
    await expect(pendingApprovalTab).toHaveAttribute('aria-selected', 'true');

    // 断言有审批中的数据
    await expect(page.getByText('RPR-2025-002')).toBeVisible({ timeout: 10000 });

    // 断言有新的 purchaseRequests 请求（状态为 PENDING_APPROVAL）
    const lastCall = msw.getLastCall('purchaseRequests');
    expect(lastCall).toBeDefined();
    expect(lastCall?.variables?.filter?.status).toBe('PENDING_APPROVAL');
  });

  test('已登录有权限用户：切换到「已批准」状态 Tab，明细数据切换', async ({
    page,
    msw,
  }) => {
    // 点击「已批准」状态 Tab
    const approvedTab = page.getByRole('tab', { name: /已批准/ });
    await approvedTab.click();
    await page.waitForTimeout(500);

    // 断言「已批准」Tab 激活
    await expect(approvedTab).toHaveAttribute('aria-selected', 'true');

    // 断言有已批准的数据
    await expect(page.getByText('RPR-2025-003')).toBeVisible({ timeout: 10000 });

    // 断言有新的 purchaseRequests 请求（状态为 APPROVED）
    const lastCall = msw.getLastCall('purchaseRequests');
    expect(lastCall).toBeDefined();
    expect(lastCall?.variables?.filter?.status).toBe('APPROVED');
  });

  test('已登录有权限用户：切换到「采购申请明细」Tab，明细表格渲染', async ({
    page,
    msw,
  }) => {
    // 点击「采购申请明细」Tab
    const detailTab = page.getByRole('tab', { name: /采购申请明细/ });
    await detailTab.click();

    // 等待明细表格加载：查找明细单号
    await expect(page.getByText('RPRI-2025-001')).toBeVisible({ timeout: 10000 });

    // 断言明细 Tab 下的状态子 Tab 在 toolbar 中（已批准 / 关闭审批中 / 已关闭）
    const approvedTab = page.locator('.ant-pro-table-list-toolbar').getByRole('tab', { name: /已批准/ });
    await expect(approvedTab).toBeVisible();

    // 断言有数据请求发出
    expect(msw.getLastCall('purchaseRequestItems')).toBeDefined();
  });

  test('已登录有权限用户：明细 Tab 的「已批准」状态下可切换采购方式子 Tab', async ({
    page,
    msw,
  }) => {
    // 切到明细 Tab
    const detailTab = page.getByRole('tab', { name: /采购申请明细/ });
    await detailTab.click();

    // 等待明细加载（默认外部采购）
    await expect(page.getByText('RPRI-2025-001')).toBeVisible({ timeout: 10000 });

    // 默认在「外部采购」子 Tab 激活
    const outerPurchaseTab = page.getByRole('tab', { name: /外部采购/ });
    await expect(outerPurchaseTab).toBeVisible();
    await expect(outerPurchaseTab).toHaveAttribute('aria-selected', 'true');

    // 切换到「委外加工」
    const outerProcessingTab = page.getByRole('tab', { name: /委外加工/ });
    await outerProcessingTab.click();
    await page.waitForTimeout(500);

    // 断言委外加工的明细数据
    await expect(page.getByText('RPRI-2025-002')).toBeVisible({ timeout: 10000 });

    // 断言有新的 purchaseRequestItems 请求（采购方式为 EP）
    const lastCall = msw.getLastCall('purchaseRequestItems');
    expect(lastCall).toBeDefined();
    expect(lastCall?.variables?.filter?.procurementMethod).toBe('EP');
  });

  test('已登录有权限用户：明细 Tab 切到「关闭审批中」状态，表格重新加载', async ({
    page,
    msw,
  }) => {
    // 切到明细 Tab
    const detailTab = page.getByRole('tab', { name: /采购申请明细/ });
    await detailTab.click();
    await page.waitForTimeout(500);

    // 切换状态到「关闭审批中」
    const closingTab = page.locator('.ant-pro-table-list-toolbar').getByRole('tab', { name: /关闭审批中/ });
    await closingTab.click();
    await page.waitForTimeout(500);

    // 断言「关闭审批中」Tab 激活
    await expect(closingTab).toHaveAttribute('aria-selected', 'true');

    // 断言有关闭中的数据
    await expect(page.getByText('RPRI-2025-003')).toBeVisible({ timeout: 10000 });

    // 断言有新的 purchaseRequestItems 请求
    const lastCall = msw.getLastCall('purchaseRequestItems');
    expect(lastCall).toBeDefined();
    expect(lastCall?.variables?.filter?.status).toBe('CLOSING');
  });
});

test.describe('无权限用户访问行为', () => {
  test('无 canCreateRootProcureRequisition 权限用户访问页面应被重定向或拒绝', async ({
    page,
    msw,
  }) => {
    // 重置为无权限用户
    await msw.reset(page, {
      id: 99,
      name: '无权限用户',
      account: 'no-perm-user',
      access: ['普通员工'], // 不含项目负责人、研发部、生产计划员、财务、管理者代表、超级管理员
      department: '其他部门',
    });

    const freshPage = await page.context().newPage();
    try {
      await msw.install(freshPage);
      await freshPage.goto('/#/Other/purchaseLedger');
      await freshPage.waitForTimeout(2000);

      // 断言：要么跳到登录页，要么 URL 仍为该页但页面显示无权限提示
      const currentUrl = freshPage.url();
      const isRedirectedToLogin = currentUrl.includes('/user/login');
      const isAccessDenied = await freshPage
        .getByText(/无访问权限|403|Access Denied/)
        .isVisible()
        .catch(() => false);

      expect(isRedirectedToLogin || isAccessDenied).toBeTruthy();
    } finally {
      await freshPage.close();
    }
  });
});

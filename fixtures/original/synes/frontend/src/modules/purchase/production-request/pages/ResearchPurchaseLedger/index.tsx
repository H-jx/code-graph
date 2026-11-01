/**
 * 研发采购申请台账页面 - 外壳
 * 路径：/Other/purchaseLedger
 *
 * 仅负责 PageContainer 包装、Tab 切换与 ProTable actionRef 共享。
 * 两个 Tab 的完整内容（请求函数、列定义、toolbar、行操作、展开行等）分别抽离到
 * 同目录的 `PurchaseRequestTabContent.tsx` 与 `PurchaseRequestItemTabContent.tsx`。
 */

import React, { useRef, useState } from 'react';
import { PageContainer } from '@ant-design/pro-components';
import type { ActionType } from '@ant-design/pro-components';
import PurchaseRequestItemTabContent from './PurchaseRequestItemTabContent';
import PurchaseRequestTabContent from './PurchaseRequestTabContent';

/** Tab key：'request' = 采购申请列表，'detail' = 采购申请明细列表 */
type LedgerTabKey = 'request' | 'detail';

/**
 * 研发采购申请台账页面组件
 */
const ResearchPurchaseLedger: React.FC = () => {
  // 当前激活的 Tab 页签
  const [activeTab, setActiveTab] = useState<LedgerTabKey>('request');

  // 跨 Tab 共享：ProTable 的 actionRef，用于 Tab 切换时触发当前 Tab 表格重新查询
  const tableActionRef = useRef<ActionType>();

  return (
    <PageContainer
      header={{
        title: false,
      }}
      onTabChange={(key) => {
        setActiveTab(key as LedgerTabKey);
        // 切换到新 Tab 时，由新 Tab 内部维护各自的展开行 / 筛选状态；
        // 外壳只需触发 ProTable 重新查询即可加载新 Tab 数据
        tableActionRef.current?.reload();
      }}
      tabActiveKey={activeTab}
      tabList={[
        { key: 'request', tab: '采购申请' },
        { key: 'detail', tab: '采购申请明细' },
      ]}
    >
      {activeTab === 'request' ? (
        <PurchaseRequestTabContent tableActionRef={tableActionRef} />
      ) : (
        <PurchaseRequestItemTabContent tableActionRef={tableActionRef} />
      )}
    </PageContainer>
  );
};

export default ResearchPurchaseLedger;

# 场景验收数据

三组场景各包含：模拟源码、流程 / SQL 材料、人工真值与期望结果。真值独立维护，
不允许被 Agent 生成的样例覆盖（生成样例与自身输出一致只能证明内部一致性）。

```text
fixtures/
├── page-location/          # 输入页面路由 → 标题、组件、操作、请求、后端方法、表字段
│   ├── source/             # 最小可核对源码（含动态 baseURL、重名组件等干扰项）
│   └── expected.json       # 人工真值：期望实体与链路
├── purchase-workflow/      # 状态、操作、前后流程与上下游
│   ├── source/
│   ├── docs/               # 已有流程说明（业务依据）
│   └── expected.json
└── purchase-ledger/        # 台账取数：字段来源、粒度、聚合、关联条件与数值
    ├── source/
    ├── sql/                # 已有查询与口径
    ├── samples.json        # 数值验收样例（输入数据 + 期望数值）
    └── expected.json
```

覆盖的验收维度（对应 `crates/core/tests/business_scenarios.rs`）：

- 页面定位、操作流程解释、台账数值；
- 误关联（跨数据源同名表、同名不同服务）；
- 源码变化后的失效与增量重跑；
- 知识过期标记与重新生成。

> 骨架阶段仅建立目录与约定，测试数据待补。

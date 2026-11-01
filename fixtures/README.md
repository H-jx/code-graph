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

当前可用：`page-location/a`（TS/TSX 前后端分仓）、`page-location/b`（JS/TSX + Python）、`purchase-workflow/{rules.md,flow.ts}`、`purchase-ledger/{schema,data,correct,fanout,where-child}.sql` 与 policy.md。

冻结依据位于 `truth/manifest.json`，逐条原始引用、expected/forbidden 身份与审阅边界位于 `truth/*.json`、`truth/review.md`；`original/synes` 保存用户提供原始项目的逐字节材料和逐文件 Git 作者/修订。真实来源声明事实、A/B 合成 smoke、独立算术审计各自分类，不混算生产业务准确率。

运行 `python scripts/verify-truth.py` 核对 50 文件与独立整数 oracle；`cargo test -p project-knowledge-core --test business_scenarios truth_` 核对出处/身份/候选写范围与内存 SQL 数值。业务插件必须通过 Core 驱动的 `frontend_` 等测试；expected 不从插件输出生成。布局与约定可以在临时副本中修改，冻结来源与真值不得就地改写。

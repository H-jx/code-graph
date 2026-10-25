# 示例：frontend-backend-a

第一种目标项目布局的四类提取器与关联器样例（占位）。

预期结构：

```text
frontend-backend-a/
├── README.md                     # 项目布局与约定说明（作为调查输入）
├── .project-knowledge/
│   ├── project.json
│   ├── conventions.md
│   └── collectors/<plugin-id>/<version>/{manifest.json,src,cases,deps}
├── web/                          # 前端：路由表、请求 wrapper、页面组件
└── api/                          # 后端：接口注册、Handler、业务方法、ORM 模型
```

包含内容（待补）：

- routes / frontend / api / database 四个 extract 插件与一个 link 插件；
- 每个插件的验收样例：正常链路、同名误关联、动态 baseURL、遗漏目标、源码变化；
- 人工真值：若干页面到表字段的期望链路。

用途：Core 与 Agent 的综合测试数据来源，同时作为外部 LLM 的第二份参考实现。

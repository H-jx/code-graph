# 示例：frontend-backend-a

TS/TSX 前端与 TS 后端分仓的独立验收样例。输入在 `fixtures/page-location/a`。

`frontend/manifest.json` 是可准备、验证的 T11 制品，包含锁定 TypeScript 6.0.3、GraphQL 16.11.0、注册约定和冻结 case。它提取静态 route/alias/i18n、组件与词法函数、JSX 事件、import/call、HTTP 客户端绑定与 GraphQL 操作、JSDoc、模块以及定时/消息/命令入口。Core 负责协议、证据与身份校验和批次事务。

制品由 `node scripts/package-example-collectors.mjs` 从公共 AST 机械逻辑和各自注册约定打包；`--check` 核对生成物。运行 `pnpm run test:plugins -- --capabilities routes,frontend,comments,modules,entrypoints` 时由 Rust Core 真正准备带锁依赖、运行插件并核对 frozen expected/forbidden。日常采集只使用已准备缓存，不导入业务项目 node_modules。

约定只描述本样例的注册表达式/字段，无目录扫描默认值。动态地址、客户端绑定、JSX handler、标题歧义、嵌套入口及语法错误返回具体 gap/partial；未知注册模式为 notConfigured，不按名称选择同名组件。GraphQL 内嵌文档引用完整原始调用范围，不伪造转义后的 token 字节位置。基础 URL 与 serviceId 必须有显式 clientToService 配置。当前不解析任意框架、外部包 re-export、模板或运行时动态注册。

这是源码审阅过的合成 smoke，不计入真实项目业务准确率。原始 synes 真值独立保存；真实接入由 Agent 调查后生成自己的插件，不将本样例安装为 SDK/Core 内置适配器。`api/` 实现 T12 的 HTTP/GraphQL 注册；`database/` 通过 Python 宿主内的 TypeScript AST 与 SQLGlot 解析明确实例、模型、DDL 和查询，带独立 hash-lock/case。`link/` 关联显式 client/service、GraphQL resolver 和查询数据源，缺绑定与多候选保持缺口/候选。前端支持 inline/i18n 标题、alias 数组和静态 proxy；动态 proxy 明确标记 bindingComplete=false。

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

后续内容（T12—T14）：

- routes / frontend / api / database 四个 extract 插件与一个 link 插件；
- 每个插件的验收样例：正常链路、同名误关联、动态 baseURL、遗漏目标、源码变化；
- 人工真值：若干页面到表字段的期望链路。

用途：Core 与 Agent 的综合测试数据来源，同时作为外部 LLM 的第二份参考实现。

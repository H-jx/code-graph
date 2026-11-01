# 示例：frontend-backend-b

JS/TSX 前端与 Python 后端的不同布局样例，输入在 `fixtures/page-location/b`。

`frontend/manifest.json` 是可运行的 T11 独立制品；使用 `screens/url/load/labelKey` 注册和 literal lazy import，区别于 A 的直接组件 import/route 表；业务语义与公共协议相同。定时任务、消息、命令在 startup.js 注册，后端位于 server/domain；制品没有写死这些目录。

与 A 使用同一 AST 机械模块、不同 conventions.json、独立 manifest/锁/case。`node scripts/package-example-collectors.mjs --check` 检查打包；`pnpm run test:plugins -- --capabilities routes,frontend,comments,modules,entrypoints` 通过 Core 执行两套冻结 case，核验全部 expected、forbidden、动态 gap 和 UTF-8 引用。覆盖范围与限制见 A 的说明。

样例明确标记为合成 smoke，不是生产业务准确率；真实项目规则留在 AI 调查生成的项目插件中。`api/` 实现 Python HTTP/GraphQL 注册；`database/` 使用 stdlib AST 与 SQLGlot 解析明确实例/模型、DDL 和静态查询。后端来源缺少 import 的全局 erp 调用保持缺口；新增带明确 import 的独立样本在 fixtures/truth-supplements/T12-database，冻结真值包未改动。`link/` 与 A 复用同一协议关联模块，输入保持独立的脚本/布局和原始证据。

预期与 `frontend-backend-a` 的差异：

- 后端为 Python AST，接口注册与 ORM 映射方式不同；
- 前端使用不同的路由组织与请求封装；
- 数据源多库并存，存在跨库同名表，用于验证误关联防护；
- 采集能力包含注释、业务模块与定时任务 / 消息消费等自定义入口。

包含内容（待补）：插件清单、验收样例、人工真值，以及 `status` 期望输出（base / collectors /
knowledge 分别报告，不允许统一显示成“完整索引完成”）。

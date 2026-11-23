# 内置 Agent

默认构建包含 Rig 0.45.0 Agent。Core-only 构建的基础索引与查询无需模型配置。

## 接入

先按 [安装指南](installation.md) 解压包含 Agent 的 CLI，并用仓库中的 [.env.example](../.env.example) 准备本地 `.env`。从源码构建时可运行 `cargo build -p project-knowledge-cli --locked`，然后将下方 `$cli` 改为 `.\target\debug\project-knowledge.exe`。

```powershell
$cli = 'C:/tools/project-knowledge/project-knowledge.exe'
$project = 'C:/workspace/my-repo'
$envFile = 'C:/tools/project-knowledge/.env'
& $cli init --root $project
$config = Join-Path $project '.project-knowledge/project.json'
& $cli index --config $config --base
& $cli agent configure --config $config --env-file $envFile --check
& $cli agent build --config $config --env-file $envFile --focus routes --goal '调查本项目的实际路由注册，生成有来源依据和验收样例的采集插件'
& $cli index --config $config --incremental
```

`.env` 使用 `.env.example` 中的四个字段，支持引号、注释和 `export`；不会修改进程环境。
配置时优先级为 CLI 参数 > 进程变量 > `.env` > 已有 agent 配置。
运行时模型/预算来自 `project.json`，凭据每次从进程变量或 `.env` 读取。
没有显式 `--env-file` 时依次选择工作目录 `.env`、配置目录 `.env`、配置目录上一级 `.env`，只读取找到的第一个文件。
密钥不写入 `project.json`，不传给插件、不进入 Prompt、运行记录或标准输出。
`.env` 和 `.env.*` 被索引硬排除，`.env.example` 可保留作配置说明。

`openai` 和 `openai-compatible` 使用 Chat Completions，`anthropic` 使用 Messages。
provider、model 与 baseUrl 不会在请求失败后自动切换；每个端点通过 `configure --check` 验证真实工具调用。
2026-10-27 实测的配置为阿里云 Token Plan、Anthropic 协议、`qwen3.8-max`。
该服务的 `/compatible-mode/v1` 对应 OpenAI；Anthropic 要用 `/apps/anthropic`，见[官方端点表](https://help.aliyun.com/en/model-studio/base-url)。

## 生成、修复与恢复

Agent 读取 Core 状态、地图、来源快照与原文，将带原始引用的调查约定交 Core 保存，然后生成独立插件。
`get-capabilities` 提供 Core 同源 manifest/input/output/case Schema 和通用 Node 模板/身份函数。
`get-capabilities` 的 `schema=identity` 可用 `kind` 与字符串槽位对象 `slots` 调 Core 计算实体 ID，用于生成 case 的 expected；模型无需猜测哈希。
只传实体 kind 时返回有序身份槽位；传关系 kind、fromId、toId、qualifiers 时由 Core 计算 relationId。
`schema=relation` 配合 `kind=transitions_to` 或 `joins` 返回 Core 实际使用的严格限定字段 Schema，避免把任意属性写进这些关系。
`export-snapshot` 提供有界 CollectorInput 和由 Core 生成的来源引用；大结果应缩小 include。
`write-candidate` 只接受当前 run 下的相对路径与文本，每次写入新修订。可设置 `includeIdentityHelper=true` 自动加入经过向量核验的 `identity.mjs`；manifest.artifacts 中需列出该文件。
验证入口通过 Core submit → prepare → validate（包括 case）→ activate；模型声称完成不能替代这些结果。

```powershell
& $cli agent repair --config $config --plugin my.plugin
& $cli agent repair --config $config --plugin my.plugin --with-skill --goal '复核原文、保留原 case，发布新版本并更新主题和 Skill'
& $cli agent resume --config $config --run agent_01234567-89ab-cdef-0123-456789abcdef
& $cli agent resume --config $config --run agent_01234567-89ab-cdef-0123-456789abcdef --goal '补充恢复指引，使用当前原始证据'
& $cli agent configure --config $config --auto-activate false
```

自动激活关闭时返回 `pending_activation`（exit 2），随后可显式 `plugin activate --candidate <id>`。
文件可分批组装，每次生成合并后的新修订，省略文件不会删除旧文件。验证失败后的首次修改才消耗一次修复预算；同一次修复可分批写文件。修复必须保留插件 ID、已有有效 case 和 fixture；不得缩小能力或输入选择器。新的已发布制品内容需要新版本。
修复 run 的首次候选自动继承已验证的活动文件，模型只需发送改动文件；`get-capabilities schema=repair,path=manifest.json` 可单独读取原 manifest。宿主仍重新计算制品哈希并执行完整验证及原 case。
默认总预算为 24 次模型请求、80 次工具调用、2 次修复、600 秒、80,000 输入字符和 8,192 输出 token。
`configure` 可显式设置模型、provider、base-url、api-key-env、max-turns、max-tool-calls、timeout-seconds、max-input-chars、max-output-tokens。
`--thinking false` 通过 Rig 的标准生成选项关闭思考，`--thinking true` 映射为 medium；不指定则沿用服务端默认值。`.env` 对应可选 `PROJECT_KNOWLEDGE_THINKING=true|false`。不支持该选项的端点会失败，不会静默忽略。
请求总数在 Rig Hook 中累计；没有隐藏 provider 自动重试。Core 子步骤受剩余总时间与取消控制。

运行记录在目标项目 `.project-knowledge/work/agent/<runId>/run.json`，保留阶段、制品 hash、validation/activation、累计预算和结构化错误。
恢复时重新核验来源与活动版本，预算和持久截止时间不重置；用尽预算必须新建 run。
`resume --goal` 追加指引并保存事件/指引摘要，不改变原预算和截止时间；`repair --with-skill` 在新修复 run 中接续知识步骤。
已有完整候选时，恢复先通过 Core 重做验证/激活；未要求 Skill 时，成功即可完成而无需再请求模型。`--with-skill` 的恢复会继续当前 run 的主题导入和 Skill 输出，沿用原预算与持久截止时间。首次候选缺失的 case 可补齐，已经存在且有效的 case 和输入保持不可改写。
激活事务提交后、run.json 更新前中断时，恢复只接纳候选哈希、validation、已提交 activation 与当前活动版本全部吻合的那一次提交；其他活动版本变更会拒绝恢复。重复恢复不会重复激活。
完成前由宿主重新调用 Core 核对当前事实新鲜度、活动制品哈希和 Skill 文件完整性；已完成 run 的恢复也复核这些产物。来源或输出发生变化时，旧 complete 报告不能直接复用。
运行目录有 OS 所有权锁；Core 工具在唯一 owned 线程串行执行，模型结束/取消/超时后停止并回收工具 owner。
约定与知识调用共享同一取消令牌和剩余截止时间；Core 在查询循环和提交前检查。原始片段、coverage、partial 状态和 gaps 都参与上下文收据摘要，检查时钟不参与。
401/403、429、端点不兼容、非法输出、缺 key 和预算用尽分别报告结构化错误，不保存 provider 原始错误正文。
成功 exit 0；失败 exit 1；partial/pending_activation exit 2；取消 exit 130。

日常 `index --incremental` 和 `collect` 仅运行已激活插件，不读取 `.env` 或调用模型。

## 约定、身份与知识

`import-conventions` 接受 notes、sourceIds；可选 identityDelta 使用同源 Core 类型，必须附上 `get-capabilities` 返回的 expectedConfigHash。
Core 校验原始证据、登记 derivedConvention 和 provenance；身份变更后先 `update-base`，重新导出并核对当前来源。
错误或遗漏配置哈希会拒绝身份增量，不写配置。

```powershell
& $cli agent build --config $config --focus routes --with-skill
```

`--with-skill` 在 Core 激活后提供三个工具，模型请求数、工具次数和截止时间继续累计：

1. `export-topic-context`：topicId、query、可选 view（search/locateFeature/page/workflow/report）、scopeIds、kinds、maxChars；返回 Core 当前上下文收据与原始引用。代码地图沿事实实体引用选择，主题名称只用于命名。声明必须引用包含实际内容的原文范围，不能仅引用函数名来支撑正文条件。
2. `import-topic`：同源 TopicArtifact，只接受本 run 已导出的 inputDigest，逐 claim 校验原始引用与推断标记，保留所有 context gaps。
3. `export-skill`：只传 name，Core 选择控制输出目录、核验新鲜度并原子发布完整 Skill，结果包含 directory 和 manifest hash。

模型只声称完成、未导入主题或未生成 Skill 时，运行仍是 partial，并记录 KNOWLEDGE_INCOMPLETE。
只有已激活且 Core 返回 fresh/complete 的 Skill 才完成；来源缺口、预算不足及生成失败都保留实际状态。
未指定 `--with-skill` 的运行不注册这三个工具。完整文件 manifest 与 CURRENT 指针规则见 docs/protocol.md。

## 当前验收范围

派生约定/identityDelta、主题与 Skill 已接入 Core；本地模拟 provider 的完整工具流与异常、预算、取消均有验证。
T18 用实际模型端点验证了查询工具。T19 在 Windows x64 基于冻结的采购流程资料验证了插件生成、有限修复、激活、主题和 Skill，以及状态、条件、有序键和原文范围。
Core-only 外部候选、无模型凭据的增量更新、CLI 终止后的恢复、预算保留和激活幂等也已验证。三个业务场景及 CLI、SDK、MCP 的对照见 [验收范围](validation/acceptance.md)。
候选自带用例始终是 candidate 来源，不能冒充 T06 独立业务真值。
工具协议和激活回归、真实模型 smoke/小型 fixture 不代表三个业务场景或所有端点已通过验收。
工程验证和真实请求摘要见 `docs/evidence/T18/`、`docs/evidence/T19/`。

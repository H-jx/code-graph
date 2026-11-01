# T06 独立真值审核状态

2026-10-26：blocked。核对当前工作区，fixtures/truth 不存在；未找到独立作者、维护者签认、原始业务资料与冻结 truthHash。计划 §5 明确其 fixture 路径、数据和数值为待创建/待独立核对的设计。

fixtures/plugins/staging、fixtures/plugins/extensions 由实现 Agent 为协议/事务测试编写。它们只证明合成 smoke 的内部一致性，不是业务 expected/forbidden 或数值口径的独立来源。不得从插件输出反推真值，也不补造审阅者/签名/准确率。

解除条件：有维护者确认的原始业务资料或独立审计样本，保存来源、作者/审阅者/日期、原始资料 hash，逐条绑定 expected/forbidden 的原始位置并独立核对数值后冻结 truthHash。收到这些输入前，继续无独立真值依赖的 Core/宿主任务。

2026-10-27 续实施复核：`fixtures/truth` 及三个业务场景目录实际存在，但都只有 `.gitkeep`；没有真值 manifest、原始业务资料或审阅记录。此前“目录不存在”应理解为没有可验收真值包，不作为准确的目录现场描述。T09/T10 的合成生命周期成果已独立推进；下一项 T11 仍依赖本任务，不能跳过前置或把合成样例改称独立真值。

2026-10-27 用户补充来源后：T06 恢复 doing。用户提供 `E:/Project/tbm/synes`；只读检查确认工作树干净，HEAD `6f47bdbf901f66e59909f8e24edd2e41a1b4f379`，提交作者 `hu-bo`，提交时间 `2026-10-20T21:58:42+08:00`。目录包含 React/Umi 前端、NestJS 后端、业务文档与历史评审材料，可继续提取独立原始依据。Git 作者信息仅为来源追溯，不代表作者已审阅本项目真值包。

用户进一步确认：真实项目应由 AI 调查并生成专属插件；SDK/Core 不固化 synes 或示例项目的路径、框架、ORM 和业务规则。原始项目仅作为调查/生成输入和独立验收来源；本轮未修改该项目、未复制其专属实现到 SDK/Core，也未将“提供路径”视为对尚未建立的真值包签认。

2026-10-28：T06 完成。原始业务资料现已冻结到 `fixtures/original/synes`，11 文件逐字节复制且逐文件保存最后提交作者、日期与 hash。Codex 在业务插件实现之前进行源码对照审阅，具体范围/限制见 `fixtures/truth/review.md`，不声称维护者签认。真实资料中的声明事实与 A/B 合成 smoke、独立审计数值样本分别标记，后两者不得计入生产业务准确率。现有 e2e 的权限注释与当前路由有冲突，已保留两份原始证据。

冻结 truthHash：`aaf1fd10b75902583be1108b083e6d3a2f71cb515f633b116ca71967f780b1ed`。50 个文件的内容 hash、逐条引用与行范围、26 个前端 expected entity/20 个 expected relation 的身份及 forbidden 对照、整数金额三行/合计、fanout 与 LEFT JOIN 反例、候选写入隔离均有可复现检查。`python docs/evidence/T06/verify.py` 实际 exit 0（Python oracle + Rust 4 passed）。真值验证不导入业务插件，冻结脚本拒绝覆盖现有包；后续变更必须单独记录原因和前后 hash。

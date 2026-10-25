# T08 — 事实校验、临时 extract/link 和范围替换

- alidation-tests.log：cargo test -p project-knowledge-core --test collectors validation_ 的输出。
  覆盖：合法输出通过；越界字节区间被拒（EVIDENCE_INVALID）；行号与字节区间不一致被拒；
  缺少证据被拒；实体 ID 与 Core 重算不一致被拒（IDENTITY_CONFLICT）；悬空关系端点被拒
  （REFERENCE_MISSING）；生成知识作为原始证据被拒；未知协议版本被拒（PROTOCOL_UNSUPPORTED）。
  错误信封按第一个问题的类别归类，后续问题视为级联。
- import-tests.log：cargo test -p project-knowledge-core --test indexing import_ 的输出。
  覆盖：同替换范围再提交批次后旧事实撤下且旧批次标记 stale 并记录失效 generation；
  替换一个生产者的批次不影响另一个生产者的断言；失败事务整体回滚且当前批次指针不变。
- 本轮同时修复了一个真实缺陷：存储层多处使用 INSERT OR IGNORE / INSERT OR REPLACE，
  会连外键违规一起吞掉，导致作用域不存在的实体被静默丢弃；已改为带显式冲突目标的 UPSERT，
  外键违规现在会真正报错。

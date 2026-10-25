# T04 — SQLite 断言存储、批次替换与一致读

- schema-ddl.txt：crates/core/migrations/0001_initial.sql 的副本（§3.4 的逻辑表契约）。
- storage-tests.log：cargo test -p project-knowledge-core --test indexing storage_ 的输出
  （6 条用例全部通过）。
- 覆盖：migration 后的表集合、PRAGMA integrity_check / oreign_key_check、
  只读打开缺失库返回 INDEX_MISSING、backup API 一致快照、批次失效单向、
  两个生产者对同一实体断言时替换其一不影响另一个、错误批次被外键拒绝、
  空批次合法、强制回滚不生效、并发读到已提交数据。

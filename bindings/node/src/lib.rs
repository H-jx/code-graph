//! Rust Core → Node.js 原生绑定（骨架）。
//!
//! 职责：转换调用与错误、传递进度与取消信号、管理 native 生命周期；
//! 不实现算法与持久化，全部转调 `project-knowledge-core`。
//!
//! 生成物由 `@napi-rs/cli` 产出：`index.js`（各平台加载器）与 `index.d.ts`（类型）。
//! 约定：查询类导出与维护类导出分开，参数与返回值使用 Core 契约生成的类型。

// TODO(pk)：在此用 #[napi] / #[napi(ts_args_type = ...)] 暴露 open/status/search/
//           locateFeature/trace/getCodeMap/readSource 以及维护类入口。

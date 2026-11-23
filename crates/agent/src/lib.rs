//! 内置 Agent：组合 Prompt、模型与 Core 工具，调查项目、生成与修复采集插件、整理业务说明。
//!
//! Agent 的职责边界：
//! - 不直接写索引表，所有注册与事实写入都经过 Core；
//! - Core、Node binding 与普通查询不依赖本 crate，因此模型框架不会传染；
//! - 初期只实现一个 Rig Agent，不建立多 Agent 调度或通用引擎适配层。

pub mod config;
pub mod prompts;
pub mod runner;
pub mod tools;

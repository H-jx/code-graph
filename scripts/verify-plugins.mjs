/**
 * 提取插件验收入口（骨架，计划 T11—T14）。
 *
 * 目标行为：由 Core 驱动 collectors/examples 下的插件对 fixtures 运行验收 case，
 * 报告 expected / forbidden / gap 的分母与差异；协议校验只在 Rust 侧实现一次，
 * 本脚本不重复实现协议逻辑。
 *
 * 目标调用：pnpm run test:plugins -- --capabilities routes,frontend,api,database
 */

console.error('TODO(pk) T11—T14: verify-plugins 未实现（需 Core 的 collect/validate 入口就绪）。');
process.exitCode = 2;

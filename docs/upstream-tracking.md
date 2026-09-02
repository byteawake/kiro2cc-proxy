# 上游仓库跟踪(TsinHzl/kiro2cc-proxy)

> 本仓库(byteawake/kiro2cc-proxy)自 v3.0.1 起与上游并行发展。
> 本文档记录上游的功能演进与最新位置,供后续合并跟进。

## 最新位置

| 项 | 值 |
|---|---|
| 上游仓库 | https://github.com/TsinHzl/kiro2cc-proxy |
| 最新 commit | **`836b43d`** |
| commit 内容 | Merge branch 'feature/account-view-supported-models' |
| 上游版本 | v3.1.0(2026-09-02) |
| 共同祖先 | `c683151`(2026-08-27,fix: 企业版 IdC 账号 getUsageLimits 403) |
| 本地记录时间 | 2026-09-02 |

## 上游独有变更(c683151..836b43d)

### 1. 账号维度模型查询(v3.1.0 主体,09-02)
- 后端:`GET /api/admin/credentials/:id/models` 路由 + `list_admin_models_for` / `list_available_models_for` 按账号查询支持模型;抽取 `acquire_token_for_id` helper 消除按 id 查询的刷新逻辑重复
- 前端:`ModelsDialog` 单账号模型弹窗 + 账号行「更多」菜单入口;模型分组纯函数抽取到 `lib/model-family.ts`

### 2. user-ui 样式全量对齐 admin-ui(v3.1.0 bump,09-01)
- user-ui 样式对齐 + 页宽改 90%;修复 CR 指出的 3 处缺陷

### 3. 并发与重试健壮性(v3.0.18,09-01)
- 退避等待前释放账号并发槽位,消除 token 计数重复编码(含 CR 契约注释)

### 4. cc-switch 导入(v3.0.17,PR #35 by lqzhgood,08-31~09-01)
- 新增 cc-switch 导入;修复浅色模式下导入按钮图标不可见
- [TOOLUSE-DIAG] 诊断日志改为异常才 warn(降噪)

### 5. token-manager ID 治理(08-31)
- 已删除账号的 ID 不再被复用,防止新账号继承历史用量记录;ID 分配 CR 修复

### 6. 小改进与 UI 修复(08-29~08-31)
- 新建 API Key 新增「无限额度」选项(默认选中);每日统计页百分比语义说明;移除支持模型页「倍率数据覆盖」环形图;启动脚本端口占用竞态修复

### 7. 文档
- README 致谢贡献者名单;CLAUDE.md 补充 OpenAI 适配层与环境变量覆盖说明

## 后续跟进

- [ ] 合并方向决策:上游 54 个提交(自 v3.0.1)与本仓库看板/截断修复(自 c683151)需双向整合
- [ ] 版本号已撞车:上游 v3.0.16~v3.1.0 与本仓库 v3.0.16~v3.1.4 同号不同义,合并后建议统一跳到新号段
- [ ] 冲突热点预判:`changelog_data.rs`(两套日志需按内容整合)、`admin-ui`(上游 ModelsDialog vs 本仓库看板页)、`Cargo.toml`
- [ ] 合并完成后更新本文档的最新 commit id

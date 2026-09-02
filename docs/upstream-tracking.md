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

- [x] 2026-09-02 已合并上游 836b43d(v3.1.0)至本仓库,合并提交 `cc8e2db`,版本统一为 **v3.2.0**
  - provider.rs:保留我方 ensure_profile_arn,采纳上游「RPM 等待先于并发 permit」
  - changelog_data.rs:上游 v3.0.17 条目与我方合并;数据看板批次归并为 v3.2.0 条目
- [x] 版本号撞车已终结:合并后统一为 v3.2.0 号段
- [ ] 上游侧(changelog 未记的 v3.1.0 功能)已在本次合并带入,后续重新以 836b43d 为基线跟踪上游新提交

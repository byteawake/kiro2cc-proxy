// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! 更新日志静态数据
//!
//! 与 `build_model_list()`（`src/anthropic/handlers.rs`）同模式：编译期硬编码，
//! 随代码发布同步维护。新增版本时在 `build_release_notes()` 顶部追加一条，
//! 并将上一条的 `is_latest` 改回 `false`。

/// 中英双语文案
#[derive(Debug, Clone)]
pub struct Bilingual {
    pub zh: String,
    pub en: String,
}

impl Bilingual {
    fn new(zh: impl Into<String>, en: impl Into<String>) -> Self {
        Self {
            zh: zh.into(),
            en: en.into(),
        }
    }
}

/// 更新日志分类分组（固定使用「新功能」「优化」「修复」三类）
#[derive(Debug, Clone)]
pub struct ReleaseNoteGroup {
    pub title: Bilingual,
    pub items: Vec<Bilingual>,
}

/// 单个版本的更新日志
#[derive(Debug, Clone)]
pub struct ReleaseNote {
    pub version: String,
    pub date: String,
    pub is_latest: bool,
    pub groups: Vec<ReleaseNoteGroup>,
}

fn feat_group(items: Vec<Bilingual>) -> ReleaseNoteGroup {
    ReleaseNoteGroup {
        title: Bilingual::new("新功能", "New Features"),
        items,
    }
}

fn improve_group(items: Vec<Bilingual>) -> ReleaseNoteGroup {
    ReleaseNoteGroup {
        title: Bilingual::new("优化", "Improvements"),
        items,
    }
}

fn fix_group(items: Vec<Bilingual>) -> ReleaseNoteGroup {
    ReleaseNoteGroup {
        title: Bilingual::new("修复", "Fixes"),
        items,
    }
}

/// 当前发布版本（来自 Cargo.toml 的 [package].version 字段），编译期常量
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 构建更新日志列表，固定按版本号从新到旧声明（不做运行时排序）
///
/// `is_latest` 由 `CURRENT_VERSION` 与条目 `version` 比对自动设定，
/// bump Cargo.toml 后无需手动改此标记。
pub fn build_release_notes() -> Vec<ReleaseNote> {
    let mut notes = vec![
        ReleaseNote {
            version: "3.1.4".to_string(),
            date: "2026-09-02".to_string(),
            is_latest: true,
            groups: vec![
                feat_group(vec![Bilingual::new(
                    "新增管理端「数据看板」：请求 / Credits / Tokens 时间序列，模型、API Key、账号三维用量切片，支持自定义时间区间与按 Key 筛选",
                    "Added an admin Data Dashboard: request/credits/tokens time series with per-model, per-key and per-credential slices, custom time ranges and per-key filtering",
                )]),
                improve_group(vec![
                    Bilingual::new(
                        "趋势图支持指标切换（请求 / Credits / Tokens），同日按小时全展示、跨天按天零填充",
                        "Trend chart gained metric switching (requests/credits/tokens); same-day ranges show all 24 hourly buckets, cross-day ranges show zero-filled daily buckets",
                    ),
                    Bilingual::new(
                        "镜像构建分层缓存 Rust 依赖，源码变更不再触发 crates.io 重新下载",
                        "Docker builds now cache crate dependencies in a separate layer; source changes no longer re-download crates.io",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "长输出不再被 max_output_tokens 截断：thinking 预算计入上游信封、默认上限对齐模型目录（64K/128K）、max_tokens 统一 1024 下限",
                        "Long outputs no longer hit max_output_tokens truncation: thinking budget added to the upstream envelope, default caps aligned with the model catalog (64K/128K), and a 1024 floor for max_tokens",
                    ),
                    Bilingual::new(
                        "上下文窗口耗尽不再误报为 max_output_tokens，改为 response.failed 如实上报，Codex 端显示压缩会话指引",
                        "Context-window exhaustion is no longer misreported as max_output_tokens; streams end with response.failed so Codex shows compaction guidance",
                    ),
                    Bilingual::new(
                        "修复看板 API Key 筛选失效（前后端参数名不一致导致过滤被静默忽略）",
                        "Fixed the dashboard API key filter (frontend/backend parameter name mismatch silently ignored the filter)",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.18".to_string(),
            date: "2026-08-31".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "Codex 客户端不解析裸 error 事件，流终止改用 response.failed 如实上报失败原因",
                        "Codex ignores bare error events; stream termination now uses response.failed to surface the real failure reason",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.17".to_string(),
            date: "2026-08-31".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "上下文窗口耗尽不再误报为 max_output_tokens，如实区分「上下文超限」与「输出截断」",
                        "context-window exhaustion is no longer misreported as max_output_tokens; it is now distinguished from output truncation",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.16".to_string(),
            date: "2026-08-31".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "长输出不再被 max_output_tokens 截断：thinking 预算计入上游信封、默认上限对齐模型目录（64K/128K）、max_tokens 统一 1024 下限",
                        "long outputs no longer truncate at max_output_tokens: thinking budget added to the upstream envelope, default caps aligned with the model catalog (64K/128K), and a 1024 floor for max_tokens",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.15".to_string(),
            date: "2026-08-27".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "修复 OpenAI 兼容端点截断流的 finish/usage 缺陷，静态模型表同步上游实时目录",
                        "fixed truncated-stream finish/usage defects on the OpenAI-compatible endpoint and synced the static model catalog with the live directory",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.13".to_string(),
            date: "2026-08-27".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "AWS SSO 设备授权导入与 IdC profileArn 自动解析",
                        "AWS SSO device-authorization import and automatic IdC profileArn resolution",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.12".to_string(),
            date: "2026-08-27".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "修复企业版 IdC 账号 getUsageLimits 返回 403 Invalid token",
                        "fixed 403 Invalid token from getUsageLimits for enterprise IdC accounts",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.11".to_string(),
            date: "2026-08-26".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "更新日志页头固定并在滚动时置顶",
                        "pinned the changelog page header on scroll",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.10".to_string(),
            date: "2026-08-25".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "额度耗尽账号在状态徽章误显示为「已禁用」",
                        "exhausted accounts no longer show a misleading disabled status badge",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.9".to_string(),
            date: "2026-08-25".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "积分消耗趋势图标注所有非零消耗日的数值",
                        "the credits trend chart now labels every non-zero day",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.8".to_string(),
            date: "2026-08-25".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "上游 HTTP 超时从 180s 提高至 1000s，修复大上下文非流式请求 502",
                        "upstream HTTP timeout raised from 180s to 1000s, fixing 502s on large non-streaming requests",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.7".to_string(),
            date: "2026-08-24".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "区分 Claude Code 与 Codex 双 Base URL 并补充兼容协议标签",
                        "separate Claude Code / Codex base URLs with compatibility-protocol badges",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "事件日志响应摘要列宽拉伸至填满右侧空白",
                        "event log response-summary column now stretches to fill the row",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.5".to_string(),
            date: "2026-08-23".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "更新日志当前版本改为动态获取与判断",
                        "current-version detection for the changelog is now dynamic",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.4".to_string(),
            date: "2026-08-23".to_string(),
            is_latest: false,
            groups: vec![
                improve_group(vec![
                    Bilingual::new(
                        "token 计数改用 tiktoken-rs 内置 cl100k_base 单例，减少重复实现",
                        "token counting now uses the tiktoken-rs built-in cl100k_base singleton to remove duplicated code",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.3".to_string(),
            date: "2026-08-23".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "引入 tiktoken-rs cl100k_base 替换本地 token 估算公式",
                        "replaced the local token estimation formula with tiktoken-rs cl100k_base",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "侧边栏折叠动画交叉淡化扩展至导航与页脚区域，消除切换卡顿",
                        "sidebar collapse cross-fade now covers nav and footer, removing the switch flicker",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.2".to_string(),
            date: "2026-08-21".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "登录页密码框移除 autoFocus，避免打开页面即触发输入法切换",
                        "removed autoFocus on the login password field to avoid IME switching on open",
                    ),
                    Bilingual::new(
                        "修复 OpenAI 流式转换孤儿 tool_call 与 SSE 溢出日志缺陷",
                        "fixed orphaned tool_call and SSE overflow logging in the OpenAI streaming conversion",
                    ),
                    Bilingual::new(
                        "输出 token 上报改为可见输出口径，超窗错误文案对齐 Anthropic 官方格式，/cc/v1 改为实时流式转发",
                        "client-facing output tokens report visible output only, context-overflow copy aligned with Anthropic, and /cc/v1 now streams in real time",
                    ),
                ]),
                improve_group(vec![
                    Bilingual::new(
                        "Admin/User 前端图标改用 Aurora Prism 方案，侧边栏 logo 路径改走 BASE_URL",
                        "Aurora Prism icon set for Admin/User frontends; sidebar logo served via BASE_URL",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "3.0.1".to_string(),
            date: "2026-08-21".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![Bilingual::new(
                    "新增 OpenAI 兼容端点 /v1/chat/completions 与 /v1/responses，支持 Codex CLI 与 OpenAI SDK 类客户端直接接入，用 Kiro 额度调用 GPT-5.6 系列模型",
                    "Added OpenAI-compatible endpoints /v1/chat/completions and /v1/responses, letting Codex CLI and OpenAI SDK clients connect directly and run GPT-5.6 models on Kiro credits",
                )]),
                fix_group(vec![Bilingual::new(
                    "修复 OpenAI 兼容端点流式转换中孤儿 tool_call 与 SSE 溢出日志的问题",
                    "Fixed orphaned tool_call handling and SSE overflow logging in the OpenAI-compatible streaming conversion",
                )]),
            ],
        },
        ReleaseNote {
            version: "2.10.2".to_string(),
            date: "2026-08-20".to_string(),
            is_latest: false,
            groups: vec![
                improve_group(vec![
                    Bilingual::new(
                        "/cc/v1/messages 改为实时流式转发，删除整段缓冲，首字延迟不再等待上游响应结束",
                        "/cc/v1/messages now forwards the stream in real time; the full-response buffer is gone, so time-to-first-token no longer waits for the upstream to finish",
                    ),
                    Bilingual::new(
                        "Admin/User 前端图标改用 Aurora Prism 方案，侧边栏 logo 随明暗主题切换",
                        "Admin/User front-end icons switched to the Aurora Prism set; the sidebar logo follows the light/dark theme",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "上报给客户端的 output_tokens 解除 380 固定上限，并按来源排除 thinking 内容",
                        "Client-facing output_tokens no longer capped at 380, and thinking content is excluded by source rather than by cap",
                    ),
                    Bilingual::new(
                        "输出 token 估算改为累加字符数后统一取整，消除逐 chunk 向上取整的累积高估",
                        "Output token estimation accumulates characters and rounds once at the end, removing the systematic overestimate from per-chunk rounding",
                    ),
                    Bilingual::new(
                        "上下文超窗错误改用 Anthropic 官方文案格式，便于客户端识别并触发自动压缩重试",
                        "Context-overflow errors now use Anthropic's official message format so clients can recognize them and trigger auto-compaction retries",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.10.1".to_string(),
            date: "2026-08-19".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "指标条新增「今日消耗积分」卡片",
                        "metrics bar gained a today's credits card",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "fallback 会话 ID 纳入首条消息避免跨会话折叠，429 不再立即解绑会话，端点全封时避让重试其它账号",
                        "fallback session IDs include the first message to avoid cross-session folding, 429 no longer unbinds sticky sessions, and full endpoint blackouts yield to other accounts",
                    ),
                    Bilingual::new(
                        "每日统计表由积分降序改为日期降序",
                        "daily stats table now sorts by date descending",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.10.0".to_string(),
            date: "2026-08-19".to_string(),
            is_latest: false,
            groups: vec![
                improve_group(vec![
                    Bilingual::new(
                        "Admin 后台全站 12 个页面按新设计稿改版，账号管理与 API Keys 改为表格化布局",
                        "All 12 Admin console pages redesigned to the new visual spec; accounts and API Keys switched to a table layout",
                    ),
                    Bilingual::new(
                        "实时日志页新增缓冲区/错误/警告指标卡、级别分段筛选与重复行折叠",
                        "Realtime logs page adds buffer/error/warning metric cards, level segment filters, and duplicate-row collapsing",
                    ),
                    Bilingual::new(
                        "每日统计页支持 7/14/30 天区间切换，趋势图仅标注峰值",
                        "Daily stats page supports 7/14/30-day range switching; the trend chart annotates peaks only",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "每日统计「今天」改按 CST(UTC+8) 计算，与后端聚合口径对齐",
                        "Daily stats now computes \"today\" in CST (UTC+8) to match the backend aggregation window",
                    ),
                    Bilingual::new(
                        "实时日志时间戳按浏览器时区渲染，不再把 UTC 时钟当本地时间显示",
                        "Realtime log timestamps render in the browser time zone instead of showing the UTC clock as local time",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.9.6".to_string(),
            date: "2026-08-13".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "Admin 后台侧边栏支持折叠/展开",
                        "the admin sidebar can now collapse/expand",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.9.5".to_string(),
            date: "2026-08-13".to_string(),
            is_latest: false,
            groups: vec![feat_group(vec![Bilingual::new(
                "Admin 后台侧边栏支持折叠/展开",
                "Admin console sidebar now supports collapse/expand",
            )])],
        },
        ReleaseNote {
            version: "2.9.4".to_string(),
            date: "2026-08-12".to_string(),
            is_latest: false,
            groups: vec![
                fix_group(vec![
                    Bilingual::new(
                        "amd64 镜像构建加重试以容忍 Docker Hub 认证服务瞬态 500",
                        "amd64 image builds retry through transient Docker Hub auth 500s",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.9.3".to_string(),
            date: "2026-08-12".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "Token 刷新失败按 invalid_grant 与瞬态失败分类处理",
                        "token refresh failures are now classified into invalid_grant vs transient",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.9.2".to_string(),
            date: "2026-08-12".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "TLS 信任本地 CA，machineId 生成失败增加兜底机制",
                        "TLS now trusts local CAs; machineId generation gained a fallback",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.9.1".to_string(),
            date: "2026-08-09".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![
                    Bilingual::new(
                        "Admin 后台支持中英文全局切换，新增更新日志页面",
                        "the admin console now switches between Chinese and English globally, with a new changelog page",
                    ),
                    Bilingual::new(
                        "支持模型页按家族分组/着色并标记最低最高费率，按模型分组新增 credits 消费统计，每日统计页新增最近 14 天 credits 趋势",
                        "models page groups/colors by family with min/max rate badges, per-model credit stats, and a 14-day credits trend on the daily stats page",
                    ),
                ]),
                improve_group(vec![
                    Bilingual::new(
                        "移除主 API Key 全局兜底认证，登录密码字段改名为 adminPsw",
                        "removed the master API key fallback auth; the login password field was renamed to adminPsw",
                    ),
                ]),
                fix_group(vec![
                    Bilingual::new(
                        "Dockerfile 补充 COPY assets，修复容器内 ip2region xdb 缺失",
                        "Dockerfile now copies assets, fixing the missing ip2region xdb in containers",
                    ),
                ]),
            ],
        },
        ReleaseNote {
            version: "2.9.0".to_string(),
            date: "2026-08-07".to_string(),
            is_latest: false,
            groups: vec![
                feat_group(vec![Bilingual::new(
                    "Admin 后台支持中英文全局切换",
                    "Admin console now supports global zh/en language switching",
                )]),
                improve_group(vec![Bilingual::new(
                    "设置页面重构为分组列表布局",
                    "Settings page redesigned with a grouped list layout",
                )]),
            ],
        },
        ReleaseNote {
            version: "2.8.25".to_string(),
            date: "2026-08-07".to_string(),
            is_latest: false,
            groups: vec![fix_group(vec![Bilingual::new(
                "支持模型列表按模型家族分组排列",
                "Supported models list is now grouped by model family",
            )])],
        },
        ReleaseNote {
            version: "2.8.24".to_string(),
            date: "2026-08-06".to_string(),
            is_latest: false,
            groups: vec![improve_group(vec![
                Bilingual::new(
                    "Admin 登录密码字段由 adminApiKey 改名为 adminPsw",
                    "Renamed the admin login password field from adminApiKey to adminPsw",
                ),
                Bilingual::new(
                    "控制台标题链接新增 hover 高亮效果",
                    "Added a hover highlight effect to the console title link",
                ),
            ])],
        },
        ReleaseNote {
            version: "2.8.23".to_string(),
            date: "2026-08-06".to_string(),
            is_latest: false,
            groups: vec![improve_group(vec![Bilingual::new(
                "移除主 API Key 全局兜底认证机制，收窄鉴权入口",
                "Removed the global fallback authentication via the master API key to narrow the authentication surface",
            )])],
        },
        ReleaseNote {
            version: "2.8.22".to_string(),
            date: "2026-08-05".to_string(),
            is_latest: false,
            groups: vec![feat_group(vec![
                Bilingual::new(
                    "支持模型页标记同家族内最低/最高费率模型",
                    "The supported models page now flags the lowest/highest priced model within each family",
                ),
                Bilingual::new(
                    "支持模型页按提供方家族着色",
                    "The supported models page is now color-coded by provider family",
                ),
                Bilingual::new(
                    "按模型分组卡片新增 credits 消费统计",
                    "Added credits consumption stats to model-grouped cards",
                ),
            ])],
        },
        ReleaseNote {
            version: "2.8.21".to_string(),
            date: "2026-08-05".to_string(),
            is_latest: false,
            groups: vec![feat_group(vec![Bilingual::new(
                "每日统计页新增最近 14 天 credits 使用趋势曲线图",
                "Added a 14-day credits usage trend chart to the daily stats page",
            )])],
        },
        ReleaseNote {
            version: "2.8.20".to_string(),
            date: "2026-08-05".to_string(),
            is_latest: false,
            groups: vec![fix_group(vec![Bilingual::new(
                "修复 Dockerfile 缺少 COPY assets 导致容器内 ip2region xdb 缺失、构建失败的问题",
                "Fixed a build failure caused by the Dockerfile missing a COPY assets step, which left the ip2region xdb file absent in the container",
            )])],
        },
    ];

    // is_latest 后处理：与 CURRENT_VERSION 匹配的条目标为最新，
    // 列表里找不到时全部置 false（避免历史版本继续被高亮）
    let matched = notes.iter().filter(|n| n.version == CURRENT_VERSION).count();
    if matched == 1 {
        for n in notes.iter_mut() {
            n.is_latest = n.version == CURRENT_VERSION;
        }
    } else {
        for n in notes.iter_mut() {
            n.is_latest = false;
        }
    }

    notes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exactly_one_latest_version() {
        let notes = build_release_notes();
        // 至多一条 is_latest=true（changelog 未列当前版本时全部 false）
        assert!(
            notes.iter().filter(|n| n.is_latest).count() <= 1,
            "is_latest=true 至多一条"
        );
        // 当前版本在 changelog 里时必须被标 latest
        if notes.iter().any(|n| n.version == CURRENT_VERSION) {
            assert!(
                notes.iter().any(|n| n.version == CURRENT_VERSION && n.is_latest),
                "当前版本 {} 在 changelog 中时必须 is_latest=true",
                CURRENT_VERSION
            );
        }
    }

    #[test]
    fn test_version_format() {
        let notes = build_release_notes();
        for note in &notes {
            assert!(
                note.version.split('.').count() == 3
                    && note
                        .version
                        .split('.')
                        .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())),
                "版本号格式非法: {}",
                note.version
            );
        }
    }

    #[test]
    fn test_bilingual_fields_non_empty() {
        let notes = build_release_notes();
        for note in &notes {
            for group in &note.groups {
                assert!(!group.title.zh.is_empty(), "分组标题 zh 不能为空");
                assert!(!group.title.en.is_empty(), "分组标题 en 不能为空");
                for item in &group.items {
                    assert!(!item.zh.is_empty(), "条目 zh 不能为空");
                    assert!(!item.en.is_empty(), "条目 en 不能为空");
                }
            }
        }
    }
}

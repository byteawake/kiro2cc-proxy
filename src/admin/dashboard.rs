// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! Admin 数据看板接口
//!
//! 聚合 `api_key_usage.json` 逐请求用量记录，输出时间序列与
//! 模型 / API Key / 账号三个维度的切片，供管理端看板页渲染。

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use chrono::{DateTime, Utc};
use serde_json::json;

use super::middleware::AdminState;
use super::types::AdminErrorResponse;

/// GET /api/admin/dashboard
///
/// 查询参数（二选一）：
/// - `hours=N`：统计最近 N 小时（1-720，默认 168）
/// - `start` + `end`：Unix 秒自定义区间（跨度钳制到 1h-8760h）
///
/// 可选 `api_key=<id>` 按 API Key 过滤（0 = 主密钥）。
/// 时间序列 ≤72h 按小时、更长按天分桶（CST），附模型 / Key / 账号切片与区间总量。
pub async fn get_dashboard(
    State(state): State<AdminState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(tracker) = &state.usage_tracker else {
        let error = AdminErrorResponse::internal_error("用量追踪未启用");
        return (StatusCode::SERVICE_UNAVAILABLE, Json(error)).into_response();
    };

    let now = Utc::now();
    let parse_ts = |v: &str| v.parse::<i64>().ok().and_then(|s| DateTime::from_timestamp(s, 0));

    let (start, end) = match (params.get("start"), params.get("end")) {
        (Some(s), Some(e)) => match (parse_ts(s), parse_ts(e)) {
            (Some(start), Some(end)) if start < end => (start, end),
            _ => {
                let error = AdminErrorResponse::bad_request(
                    "无效时间区间：start/end 须为 Unix 秒且 start < end",
                );
                return (StatusCode::BAD_REQUEST, Json(error)).into_response();
            }
        },
        _ => {
            let hours = params
                .get("hours")
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(168)
                .clamp(1, 720);
            (now - chrono::Duration::hours(hours), now)
        }
    };
    // 跨度上限 1 年，防止超大区间拖垮聚合
    let (start, end) = if end - start > chrono::Duration::hours(8760) {
        (end - chrono::Duration::hours(8760), end)
    } else {
        (start, end)
    };

    let api_key_id = match params.get("api_key").map(String::as_str) {
        None | Some("") | Some("all") => None,
        Some(v) => match v.parse::<u32>() {
            Ok(id) => Some(id),
            Err(_) => {
                let error = AdminErrorResponse::bad_request("无效 api_key：须为无符号整数 ID");
                return (StatusCode::BAD_REQUEST, Json(error)).into_response();
            }
        },
    };

    let labels = state.service.credential_labels();
    let mut snapshot =
        serde_json::to_value(tracker.get_dashboard_snapshot(start, end, api_key_id, &labels))
            .unwrap_or(serde_json::Value::Null);
    if let Some(obj) = snapshot.as_object_mut() {
        obj.insert("generatedAt".into(), json!(now));
    }
    Json(snapshot).into_response()
}

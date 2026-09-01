// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! Admin 数据看板接口
//!
//! 聚合 `api_key_usage.json` 逐请求用量记录，输出时间序列与
//! 模型 / API Key / 账号三个维度的切片，供管理端看板页渲染。

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

use super::middleware::AdminState;
use super::types::AdminErrorResponse;

/// GET /api/admin/dashboard?hours=168
///
/// 聚合最近 `hours` 小时（1-720，默认 168）的用量：
/// 时间序列（≤72h 按小时、更长按天分桶，CST）+ 模型 / Key / 账号切片 + 区间总量。
pub async fn get_dashboard(
    State(state): State<AdminState>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let Some(tracker) = &state.usage_tracker else {
        let error = AdminErrorResponse::internal_error("用量追踪未启用");
        return (axum::http::StatusCode::SERVICE_UNAVAILABLE, Json(error)).into_response();
    };
    let hours = params
        .get("hours")
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(168);
    let labels = state.service.credential_labels();
    let mut snapshot = serde_json::to_value(tracker.get_dashboard_snapshot(hours, &labels))
        .unwrap_or(serde_json::Value::Null);
    if let Some(obj) = snapshot.as_object_mut() {
        obj.insert("generatedAt".into(), json!(chrono::Utc::now()));
    }
    Json(snapshot).into_response()
}

// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! AWS SSO OIDC 自动账号导入 —— 会话管理
//!
//! 流程：
//! 1. 用户提交 Start URL / Auth Region / API Region（及可选元数据），后端注册 OIDC
//!    客户端并发起设备授权，返回带 user code 的验证 URL 给用户。
//! 2. 用户在浏览器完成 SSO 登录并批准，后台任务轮询 CreateToken。
//! 3. 批准后取得 Refresh Token / Client ID / Client Secret，结合锁定的 Region，
//!    走既有的添加账号流程（`MultiTokenManager::add_credential`，IdC 认证）入库，
//!    随后预解析真实 profileArn 并拉取订阅等级。
//!
//! 从提交到完成，所有请求数据（Start URL / Region / 元数据）均在后端锁定，
//! 客户端后续仅凭 session id 轮询状态，无法改变任何参数。

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration, Utc};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

use crate::common::jwt;
use crate::http_client::ProxyConfig;
use crate::kiro::model::credentials::KiroCredentials;
use crate::kiro::sso_oidc::{self, CreateTokenPoll};
use crate::kiro::token_manager::MultiTokenManager;
use crate::model::config::{Config, TlsBackend};

/// 支持的 API Region（选择框可选值）
pub const ALLOWED_API_REGIONS: [&str; 2] = ["us-east-1", "eu-central-1"];

/// 注册 OIDC 客户端时额外请求的身份声明 scopes。
///
/// 带 `openid` 等声明 scope 注册时，CreateToken 会返回 idToken（JWT），
/// 载荷含 email / preferred_username 等身份信息，用于自动回填邮箱与昵称；
/// 部分上游拒绝组合 scope，届时回退到配置的基础 scopes 重试（仅少拿身份，不影响导入）。
const IDENTITY_CLAIM_SCOPES: [&str; 3] = ["openid", "profile", "email"];

/// 设备授权默认有效期（秒），服务端未返回时使用
const DEFAULT_DEVICE_EXPIRES_IN: i64 = 600;
/// 会话硬超时上限（秒），防止后台任务无限轮询
const MAX_SESSION_TTL_SECS: i64 = 900;
/// 默认轮询间隔（秒），服务端未返回 interval 时使用
const DEFAULT_POLL_INTERVAL_SECS: i64 = 5;
/// slow_down 时的退避增量（秒）
const SLOW_DOWN_BACKOFF_SECS: i64 = 5;
/// 已结束会话保留时长（秒），超过则在下次创建时清理
const FINISHED_SESSION_RETENTION_SECS: i64 = 3600;

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SsoStatus {
    /// 等待用户在浏览器完成登录并批准
    Pending,
    /// 已批准并成功添加账号
    Completed,
    /// 失败（网络错误、添加账号失败等）
    Failed,
    /// 设备码已过期（用户未在时限内批准）
    Expired,
    /// 用户拒绝了授权
    Denied,
    /// 会话被取消
    Cancelled,
}

impl SsoStatus {
    fn is_finished(self) -> bool {
        !matches!(self, SsoStatus::Pending)
    }
}

/// 单个 SSO 会话的完整状态（后端锁定，客户端不可修改）
#[derive(Debug, Clone)]
struct SsoSessionState {
    status: SsoStatus,
    // ==== 锁定参数（创建后不可变） ====
    start_url: String,
    auth_region: String,
    api_region: String,
    // ==== 展示给用户的设备授权信息 ====
    user_code: String,
    verification_uri: Option<String>,
    verification_uri_complete: Option<String>,
    // ==== 时间 ====
    created_at: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    // ==== 结果 ====
    credential_id: Option<u64>,
    email: Option<String>,
    error: Option<String>,
}

/// 会话条目（状态 + 取消标志）
struct SsoSession {
    state: SsoSessionState,
    cancel: Arc<AtomicBool>,
}

/// 会话状态响应（对外，仅暴露非敏感字段；不含 clientSecret / refreshToken）
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SsoSessionResponse {
    pub session_id: String,
    pub status: SsoStatus,
    pub start_url: String,
    pub auth_region: String,
    pub api_region: String,
    pub user_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_uri_complete: Option<String>,
    pub expires_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 发起会话请求
///
/// 除 Start URL / Region 外，还接受与「手动添加账号」一致的可选元数据
/// （邮箱 / 备注名 / 账号级代理），导入产生的记录与手动添加字段完全对齐。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSsoSessionRequest {
    /// 门户 Start URL，例如 https://<alias>.awsapps.com/start
    pub start_url: String,
    /// 门户 / SSO 所在区域（用于 OIDC 刷新）
    pub auth_region: String,
    /// API Region（用于 API 请求），仅允许 us-east-1 / eu-central-1
    pub api_region: String,
    /// 优先级（可选，默认 0）
    #[serde(default)]
    pub priority: u32,
    /// 用户邮箱（可选，仅用于面板展示）
    #[serde(default)]
    pub email: Option<String>,
    /// 备注名（可选）
    #[serde(default)]
    pub nickname: Option<String>,
    /// 账号级代理 URL（可选；SSO 流程本身走全局代理，此代理从验活刷新开始生效）
    #[serde(default)]
    pub proxy_url: Option<String>,
    /// 账号级代理认证用户名（可选）
    #[serde(default)]
    pub proxy_username: Option<String>,
    /// 账号级代理认证密码（可选）
    #[serde(default)]
    pub proxy_password: Option<String>,
}

/// 校验并规范化发起参数：`(start_url, auth_region, api_region)`
fn validate_start_request(
    start_url: &str,
    auth_region: &str,
    api_region: &str,
) -> Result<(String, String, String), String> {
    let start_url = start_url.trim().trim_end_matches('/').to_string();
    let auth_region = auth_region.trim().to_string();
    let api_region = api_region.trim().to_string();

    if start_url.is_empty() {
        return Err("Start URL 不能为空".to_string());
    }
    if !(start_url.starts_with("https://") || start_url.starts_with("http://")) {
        return Err("Start URL 必须以 http(s):// 开头".to_string());
    }
    if auth_region.is_empty() {
        return Err("Auth Region 不能为空".to_string());
    }
    if !ALLOWED_API_REGIONS.contains(&api_region.as_str()) {
        return Err(format!("API Region 仅支持 {:?}", ALLOWED_API_REGIONS));
    }
    Ok((start_url, auth_region, api_region))
}

/// 从配置重建全局代理（与 main.rs 的全局代理构建方式一致）。
///
/// 仅用于 OIDC 流程本身；账号级代理在凭据入库后的验活刷新起生效。
fn global_proxy(config: &Config) -> Option<ProxyConfig> {
    config.proxy_url.as_ref().map(|url| {
        let mut proxy = ProxyConfig::new(url);
        if let (Some(username), Some(password)) = (&config.proxy_username, &config.proxy_password) {
            proxy = proxy.with_auth(username, password);
        }
        proxy
    })
}

/// 注册 OIDC 客户端：优先带身份声明 scopes（换取 idToken），被拒则回退基础 scopes。
///
/// 两级注册都只是客户端登记，不产生用户可见影响；返回值与所用 scopes 的成功路径无关。
async fn register_client_prefer_identity(
    auth_region: &str,
    config_scopes: &[String],
    proxy: Option<&ProxyConfig>,
    tls_backend: TlsBackend,
) -> anyhow::Result<sso_oidc::RegisterClientResponse> {
    let mut with_identity = config_scopes.to_vec();
    for s in IDENTITY_CLAIM_SCOPES {
        with_identity.push(s.to_string());
    }

    match sso_oidc::register_client(auth_region, &with_identity, proxy, tls_backend).await {
        Ok(resp) => {
            tracing::info!("SSO 客户端注册已启用身份声明 scopes（自动回填邮箱/昵称）");
            Ok(resp)
        }
        Err(first_err) => {
            tracing::warn!(
                "带身份声明 scopes 注册被拒（{}），回退基础 scopes",
                first_err
            );
            sso_oidc::register_client(auth_region, config_scopes, proxy, tls_backend).await
        }
    }
}

/// SSO 会话管理器
pub struct SsoSessionManager {
    token_manager: Arc<MultiTokenManager>,
    sessions: Arc<Mutex<HashMap<String, SsoSession>>>,
}

/// 会话操作错误
#[derive(Debug)]
pub enum SsoError {
    /// 请求参数无效
    InvalidRequest(String),
    /// 会话不存在
    NotFound(String),
    /// 上游 OIDC 调用失败
    Upstream(String),
}

/// 后台轮询任务的锁定参数 + 用户提交的可选元数据
struct PollContext {
    session_id: String,
    token_manager: Arc<MultiTokenManager>,
    sessions: Arc<Mutex<HashMap<String, SsoSession>>>,
    cancel: Arc<AtomicBool>,
    auth_region: String,
    api_region: String,
    start_url: String,
    client_id: String,
    client_secret: String,
    device_code: String,
    interval: i64,
    deadline: DateTime<Utc>,
    request: StartSsoSessionRequest,
    proxy: Option<ProxyConfig>,
    tls_backend: TlsBackend,
}

impl SsoSessionManager {
    pub fn new(token_manager: Arc<MultiTokenManager>) -> Self {
        Self {
            token_manager,
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// 清理已结束且超过保留时长的会话
    fn purge_finished(&self) {
        let now = Utc::now();
        let mut sessions = self.sessions.lock();
        sessions.retain(|_, s| {
            if !s.state.status.is_finished() {
                return true;
            }
            (now - s.state.created_at).num_seconds() < FINISHED_SESSION_RETENTION_SECS
        });
    }

    /// 生成会话状态响应
    fn to_response(session_id: &str, state: &SsoSessionState) -> SsoSessionResponse {
        SsoSessionResponse {
            session_id: session_id.to_string(),
            status: state.status,
            start_url: state.start_url.clone(),
            auth_region: state.auth_region.clone(),
            api_region: state.api_region.clone(),
            user_code: state.user_code.clone(),
            verification_uri: state.verification_uri.clone(),
            verification_uri_complete: state.verification_uri_complete.clone(),
            expires_at: state.expires_at.to_rfc3339(),
            credential_id: state.credential_id,
            email: state.email.clone(),
            error: state.error.clone(),
        }
    }

    /// 发起 SSO 会话：注册客户端 + 设备授权，返回带 user code 的验证 URL
    pub async fn start_session(
        &self,
        req: StartSsoSessionRequest,
    ) -> Result<SsoSessionResponse, SsoError> {
        // 清理陈旧会话
        self.purge_finished();

        let (start_url, auth_region, api_region) = validate_start_request(
            &req.start_url,
            &req.auth_region,
            &req.api_region,
        )
        .map_err(SsoError::InvalidRequest)?;

        let config = self.token_manager.config();
        let tls_backend = config.tls_backend;
        let proxy = global_proxy(config);
        let scopes = config.sso_scopes.clone();

        // 1. RegisterClient（CodeWhisperer scopes 保证 Token 可访问 Kiro API；
        //    追加 openid 等声明 scopes 以便拿到带邮箱的 idToken，被拒则回退）
        let registered = register_client_prefer_identity(
            &auth_region,
            &scopes,
            proxy.as_ref(),
            tls_backend,
        )
        .await
        .map_err(|e| SsoError::Upstream(e.to_string()))?;

        // 2. StartDeviceAuthorization
        let dev = sso_oidc::start_device_authorization(
            &auth_region,
            &registered.client_id,
            &registered.client_secret,
            &start_url,
            proxy.as_ref(),
            tls_backend,
        )
        .await
        .map_err(|e| SsoError::Upstream(e.to_string()))?;

        let now = Utc::now();
        let device_expires_in = dev.expires_in.unwrap_or(DEFAULT_DEVICE_EXPIRES_IN);
        // 会话截止时间取设备码有效期与硬上限的较小值
        let ttl = device_expires_in.min(MAX_SESSION_TTL_SECS).max(1);
        let expires_at = now + Duration::seconds(ttl);
        let interval = dev.interval.unwrap_or(DEFAULT_POLL_INTERVAL_SECS).max(1);

        let session_id = uuid::Uuid::new_v4().to_string();
        let cancel = Arc::new(AtomicBool::new(false));

        let state = SsoSessionState {
            status: SsoStatus::Pending,
            start_url: start_url.clone(),
            auth_region: auth_region.clone(),
            api_region: api_region.clone(),
            user_code: dev.user_code.clone(),
            verification_uri: dev.verification_uri.clone(),
            verification_uri_complete: dev.verification_uri_complete.clone(),
            created_at: now,
            expires_at,
            credential_id: None,
            email: req.email.clone(),
            error: None,
        };

        let response = Self::to_response(&session_id, &state);

        {
            let mut sessions = self.sessions.lock();
            sessions.insert(
                session_id.clone(),
                SsoSession {
                    state,
                    cancel: cancel.clone(),
                },
            );
        }

        // 3. 后台轮询 CreateToken 并在批准后添加账号
        let poll_ctx = PollContext {
            session_id,
            token_manager: self.token_manager.clone(),
            sessions: self.sessions.clone(),
            cancel,
            auth_region,
            api_region,
            start_url,
            client_id: registered.client_id,
            client_secret: registered.client_secret,
            device_code: dev.device_code,
            interval,
            deadline: expires_at,
            request: req,
            proxy,
            tls_backend,
        };
        tokio::spawn(poll_and_import(poll_ctx));

        Ok(response)
    }

    /// 查询会话状态
    pub fn get_session(&self, session_id: &str) -> Result<SsoSessionResponse, SsoError> {
        let sessions = self.sessions.lock();
        let session = sessions
            .get(session_id)
            .ok_or_else(|| SsoError::NotFound(format!("会话不存在: {}", session_id)))?;
        Ok(Self::to_response(session_id, &session.state))
    }

    /// 取消会话（若仍在等待中）
    pub fn cancel_session(&self, session_id: &str) -> Result<SsoSessionResponse, SsoError> {
        let mut sessions = self.sessions.lock();
        let session = sessions
            .get_mut(session_id)
            .ok_or_else(|| SsoError::NotFound(format!("会话不存在: {}", session_id)))?;

        if !session.state.status.is_finished() {
            session.cancel.store(true, Ordering::Relaxed);
            session.state.status = SsoStatus::Cancelled;
        }
        Ok(Self::to_response(session_id, &session.state))
    }
}

/// 更新会话状态的辅助函数
fn set_session_status(
    sessions: &Arc<Mutex<HashMap<String, SsoSession>>>,
    session_id: &str,
    update: impl FnOnce(&mut SsoSessionState),
) {
    let mut map = sessions.lock();
    if let Some(session) = map.get_mut(session_id) {
        // 已被取消的会话不再覆盖状态
        if session.state.status == SsoStatus::Cancelled {
            return;
        }
        update(&mut session.state);
    }
}

/// 用取得的 Token 构建 IdC 凭据（纯逻辑，便于单测）
///
/// 可选元数据（邮箱/备注/代理）原样透传，与「手动添加账号」落库字段保持一致。
fn build_idc_credential(
    refresh_token: String,
    client_id: String,
    client_secret: String,
    auth_region: String,
    api_region: String,
    req: &StartSsoSessionRequest,
) -> KiroCredentials {
    // 昵称缺省时按产品规则取邮箱（「用户名即邮箱」），保证列表可读
    let nickname = req
        .nickname
        .clone()
        .or_else(|| req.email.clone());
    KiroCredentials {
        auth_method: Some("idc".to_string()),
        refresh_token: Some(refresh_token),
        client_id: Some(client_id),
        client_secret: Some(client_secret),
        auth_region: Some(auth_region),
        api_region: Some(api_region),
        priority: req.priority,
        email: req.email.clone(),
        nickname,
        proxy_url: req.proxy_url.clone(),
        proxy_username: req.proxy_username.clone(),
        proxy_password: req.proxy_password.clone(),
        ..Default::default()
    }
}

/// 后台轮询 CreateToken，批准后添加账号
async fn poll_and_import(mut ctx: PollContext) {
    let mut interval = ctx.interval;

    let token = loop {
        // 取消检查
        if ctx.cancel.load(Ordering::Relaxed) {
            tracing::info!("SSO 会话 {} 已取消，停止轮询", ctx.session_id);
            return;
        }

        // 超时检查
        if Utc::now() >= ctx.deadline {
            tracing::warn!("SSO 会话 {} 超时", ctx.session_id);
            set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                s.status = SsoStatus::Expired;
                s.error = Some("等待用户授权超时".to_string());
            });
            return;
        }

        // 等待一个轮询间隔（分片睡眠以便及时响应取消/超时）
        let mut slept = 0i64;
        while slept < interval {
            if ctx.cancel.load(Ordering::Relaxed) || Utc::now() >= ctx.deadline {
                break;
            }
            tokio::time::sleep(StdDuration::from_secs(1)).await;
            slept += 1;
        }

        if ctx.cancel.load(Ordering::Relaxed) {
            return;
        }

        match sso_oidc::create_token_once(
            &ctx.auth_region,
            &ctx.client_id,
            &ctx.client_secret,
            &ctx.device_code,
            ctx.proxy.as_ref(),
            ctx.tls_backend,
        )
        .await
        {
            Ok(CreateTokenPoll::Token(token)) => break *token,
            Ok(CreateTokenPoll::Pending) => continue,
            Ok(CreateTokenPoll::SlowDown) => {
                interval += SLOW_DOWN_BACKOFF_SECS;
                continue;
            }
            Ok(CreateTokenPoll::Expired) => {
                set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                    s.status = SsoStatus::Expired;
                    s.error = Some("设备码已过期，请重新发起".to_string());
                });
                return;
            }
            Ok(CreateTokenPoll::Denied) => {
                set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                    s.status = SsoStatus::Denied;
                    s.error = Some("用户拒绝了授权".to_string());
                });
                return;
            }
            Err(e) => {
                tracing::error!("SSO 会话 {} 轮询失败: {}", ctx.session_id, e);
                set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                    s.status = SsoStatus::Failed;
                    s.error = Some(format!("轮询失败: {}", e));
                });
                return;
            }
        }
    };

    // 已批准，取得 refresh token；再次检查取消
    if ctx.cancel.load(Ordering::Relaxed) {
        return;
    }

    // 解析 idToken 身份声明，自动回填邮箱 / 昵称（仅当表单对应字段为空时生效，
    // 显式填写优先；昵称在无独立用户名时按既定规则兜底为邮箱）。
    if let Some(payload) = token.id_token.as_deref().and_then(jwt::decode_jwt_payload) {
        let identity = jwt::extract_identity(&payload);
        if ctx.request.email.is_none() {
            ctx.request.email = identity.email.clone();
        }
        if ctx.request.nickname.is_none() {
            ctx.request.nickname = identity
                .display_name
                .or_else(|| ctx.request.email.clone());
        }
        tracing::info!(
            "SSO 会话 {} 已从 idToken 解析身份：email={:?} displayName={:?}",
            ctx.session_id,
            ctx.request.email,
            ctx.request.nickname
        );
    } else if ctx.request.nickname.is_none() && ctx.request.email.is_some() {
        // 无声明 scope 拿不到 idToken：昵称仍按规则兜底为邮箱
        ctx.request.nickname = ctx.request.email.clone();
    }

    let refresh_token = match token.refresh_token {
        Some(rt) if !rt.is_empty() => rt,
        _ => {
            set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                s.status = SsoStatus::Failed;
                s.error = Some("授权成功但未返回 Refresh Token".to_string());
            });
            return;
        }
    };

    // 通过既有的添加账号流程入库（内部会刷新 Token 验证有效性）
    let new_cred = build_idc_credential(
        refresh_token,
        ctx.client_id.clone(),
        ctx.client_secret,
        ctx.auth_region.clone(),
        ctx.api_region.clone(),
        &ctx.request,
    );

    match ctx.token_manager.add_credential(new_cred).await {
        Ok(credential_id) => {
            // 预解析真实 profileArn 并落盘（失败不影响导入）。
            //
            // AWS SSO OIDC 的 /token 不返回 profileArn，Enterprise / IdC 账号不带
            // 真实 ARN 调流式端点会被拒 403。这里提前解析一次让账号一导入就可用；
            // 漏掉也没关系——KiroProvider::ensure_profile_arn 会在首次请求前兜底。
            match ctx.token_manager.resolve_profile_arn_for_id(credential_id).await {
                Ok(Some(arn)) => {
                    tracing::info!("SSO 导入后已解析 profileArn: 账号 #{} → {}", credential_id, arn)
                }
                Ok(None) => tracing::info!(
                    "账号 #{} 无 Enterprise profile（BuilderID 等），将使用占位符 ARN",
                    credential_id
                ),
                Err(e) => tracing::warn!("SSO 导入后解析 profileArn 失败（不影响导入）: {}", e),
            }

            // 主动获取订阅等级（失败不影响导入）
            if let Err(e) = ctx.token_manager.get_usage_limits_for(credential_id).await {
                tracing::warn!("SSO 导入后获取订阅等级失败（不影响导入）: {}", e);
            }
            tracing::info!(
                "SSO 会话 {} 完成，已添加账号 #{}（start_url={}）",
                ctx.session_id,
                credential_id,
                ctx.start_url
            );
            set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                s.status = SsoStatus::Completed;
                s.credential_id = Some(credential_id);
                if let Some(email) = &ctx.request.email {
                    s.email = Some(email.clone());
                }
            });
        }
        Err(e) => {
            tracing::error!("SSO 会话 {} 添加账号失败: {}", ctx.session_id, e);
            set_session_status(&ctx.sessions, &ctx.session_id, |s| {
                s.status = SsoStatus::Failed;
                s.error = Some(format!("添加账号失败: {}", e));
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_request() -> StartSsoSessionRequest {
        StartSsoSessionRequest {
            start_url: "https://d-1234567890.awsapps.com/start".to_string(),
            auth_region: "us-east-1".to_string(),
            api_region: "us-east-1".to_string(),
            priority: 0,
            email: None,
            nickname: None,
            proxy_url: None,
            proxy_username: None,
            proxy_password: None,
        }
    }

    #[test]
    fn test_sso_status_is_finished() {
        assert!(!SsoStatus::Pending.is_finished());
        assert!(SsoStatus::Completed.is_finished());
        assert!(SsoStatus::Failed.is_finished());
        assert!(SsoStatus::Expired.is_finished());
        assert!(SsoStatus::Denied.is_finished());
        assert!(SsoStatus::Cancelled.is_finished());
    }

    #[test]
    fn test_validate_start_request_ok_and_trims() {
        let (url, region, api) = validate_start_request(
            "https://example.awsapps.com/start/",
            " eu-central-1 ",
            "eu-central-1",
        )
        .unwrap();
        assert_eq!(url, "https://example.awsapps.com/start");
        assert_eq!(region, "eu-central-1");
        assert_eq!(api, "eu-central-1");
    }

    #[test]
    fn test_validate_start_request_rejects_bad_input() {
        assert!(validate_start_request("", "us-east-1", "us-east-1").is_err());
        assert!(validate_start_request("ftp://x", "us-east-1", "us-east-1").is_err());
        assert!(validate_start_request("https://x.awsapps.com/start", "", "us-east-1").is_err());
        assert!(validate_start_request("https://x.awsapps.com/start", "us-east-1", "").is_err());
        assert!(validate_start_request("https://x", "us-east-1", "ap-southeast-1").is_err());
    }

    #[test]
    fn test_allowed_api_regions() {
        assert_eq!(ALLOWED_API_REGIONS, ["us-east-1", "eu-central-1"]);
    }

    #[test]
    fn test_build_idc_credential_passthrough_metadata() {
        let mut req = base_request();
        req.priority = 7;
        req.email = Some("user@example.com".to_string());
        req.nickname = Some("企业号".to_string());
        req.proxy_url = Some("http://127.0.0.1:7890".to_string());
        req.proxy_username = Some("u".to_string());
        req.proxy_password = Some("p".to_string());

        let cred = build_idc_credential(
            "refresh-token".to_string(),
            "client-id".to_string(),
            "client-secret".to_string(),
            "us-east-1".to_string(),
            "eu-central-1".to_string(),
            &req,
        );

        assert_eq!(cred.auth_method.as_deref(), Some("idc"));
        assert_eq!(cred.refresh_token.as_deref(), Some("refresh-token"));
        assert_eq!(cred.client_id.as_deref(), Some("client-id"));
        assert_eq!(cred.client_secret.as_deref(), Some("client-secret"));
        assert_eq!(cred.auth_region.as_deref(), Some("us-east-1"));
        assert_eq!(cred.api_region.as_deref(), Some("eu-central-1"));
        assert_eq!(cred.priority, 7);
        assert_eq!(cred.email.as_deref(), Some("user@example.com"));
        assert_eq!(cred.nickname.as_deref(), Some("企业号"));
        assert_eq!(cred.proxy_url.as_deref(), Some("http://127.0.0.1:7890"));
        assert_eq!(cred.proxy_username.as_deref(), Some("u"));
        assert_eq!(cred.proxy_password.as_deref(), Some("p"));
        // 未设置的敏感/账号信息不应残留
        assert_eq!(cred.access_token, None);
        assert_eq!(cred.profile_arn, None);
    }

    #[test]
    fn test_build_idc_credential_defaults_without_metadata() {
        let req = base_request();
        let cred = build_idc_credential(
            "rt".to_string(),
            "ci".to_string(),
            "cs".to_string(),
            "us-east-1".to_string(),
            "us-east-1".to_string(),
            &req,
        );
        assert_eq!(cred.priority, 0);
        assert_eq!(cred.email, None);
        assert_eq!(cred.nickname, None);
        assert_eq!(cred.proxy_url, None);
    }

    #[test]
    fn test_build_idc_credential_nickname_defaults_to_email() {
        // 产品规则：昵称缺省时取邮箱（「用户名即邮箱」）
        let mut req = base_request();
        req.email = Some("someone@corp.com".to_string());
        let cred = build_idc_credential(
            "rt".to_string(),
            "ci".to_string(),
            "cs".to_string(),
            "us-east-1".to_string(),
            "us-east-1".to_string(),
            &req,
        );
        assert_eq!(cred.email.as_deref(), Some("someone@corp.com"));
        assert_eq!(cred.nickname.as_deref(), Some("someone@corp.com"));

        // 显式昵称不被覆盖
        req.nickname = Some("自定义名".to_string());
        let cred = build_idc_credential(
            "rt".to_string(),
            "ci".to_string(),
            "cs".to_string(),
            "us-east-1".to_string(),
            "us-east-1".to_string(),
            &req,
        );
        assert_eq!(cred.nickname.as_deref(), Some("自定义名"));
    }

    #[test]
    fn test_identity_claim_scopes_constant() {
        assert_eq!(IDENTITY_CLAIM_SCOPES, ["openid", "profile", "email"]);
    }

    #[test]
    fn test_response_hides_sensitive_fields() {
        // 响应结构体只含展示字段：序列化结果不得出现任何 secret/token 字段
        let state = SsoSessionState {
            status: SsoStatus::Pending,
            start_url: "https://x".to_string(),
            auth_region: "us-east-1".to_string(),
            api_region: "us-east-1".to_string(),
            user_code: "ABCD-EFGH".to_string(),
            verification_uri: Some("https://view.awsapps.com/start#/device".to_string()),
            verification_uri_complete: Some(
                "https://view.awsapps.com/start#/device?user_code=ABCD-EFGH".to_string(),
            ),
            created_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(600),
            credential_id: None,
            email: Some("user@example.com".to_string()),
            error: None,
        };
        let resp = SsoSessionManager::to_response("sid", &state);
        let json = serde_json::to_string(&resp).unwrap();

        assert!(json.contains("\"userCode\":\"ABCD-EFGH\""));
        assert!(json.contains("\"verificationUriComplete\""));
        assert!(json.contains("\"status\":\"pending\""));
        assert!(!json.contains("secret"));
        assert!(!json.contains("refresh_token") && !json.contains("refreshToken"));
        assert!(!json.contains("clientSecret"));
    }

    #[test]
    fn test_global_proxy_with_auth() {
        let mut config = Config::default();
        config.proxy_url = Some("socks5://gate:1080".to_string());
        let proxy = global_proxy(&config).unwrap();
        assert_eq!(proxy, ProxyConfig::new("socks5://gate:1080"));

        config.proxy_username = Some("u".to_string());
        config.proxy_password = Some("p".to_string());
        let proxy = global_proxy(&config).unwrap();
        assert_eq!(
            proxy,
            ProxyConfig::new("socks5://gate:1080").with_auth("u", "p")
        );
    }

    #[test]
    fn test_global_proxy_none_when_unconfigured() {
        let config = Config::default();
        assert!(global_proxy(&config).is_none());
    }
}

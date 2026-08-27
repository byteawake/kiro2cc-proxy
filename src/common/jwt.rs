// Copyright (c) 2026 Harllan He. Licensed under MIT.
//! JWT 载荷解码与身份声明提取
//!
//! 仅用于把 OIDC Token（AWS SSO 的 idToken / external_idp 的 id_token 或
//! access token）中的身份信息回填到账号的展示字段（邮箱 / 昵称）。
//! **不校验签名**——这些字段只是展示用途，且 Token 本身已在 TLS 信道内
//! 直接来自上游，无需防篡改校验。

use base64::Engine;
use serde_json::Value;

/// 从身份声明中提取出的用户标识
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TokenIdentity {
    /// 邮箱：`email` 声明优先，否则取第一个含 `@` 的用户名型声明
    /// （preferred_username / unique_name 在 Entra ID 等平台常直接是邮箱）
    pub email: Option<String>,
    /// 展示名（用户名）：name > preferred_username > unique_name > given+family 拼接
    pub display_name: Option<String>,
}

/// 解码 JWT 载荷（三段式 token 的第二段，base64url 编码，无 / 有 padding 均可）。
///
/// 非法输入（段数不对、非 base64、非 JSON 对象）一律返回 `None`。
pub fn decode_jwt_payload(token: &str) -> Option<Value> {
    let mut parts = token.split('.');
    let payload = match (parts.next(), parts.next(), parts.next()) {
        (Some(_), Some(p), Some(_)) if !p.is_empty() => p,
        _ => return None,
    };

    // base64url 无 padding；`URL_SAFE` 引擎要求 padding，用 NO_PAD 失败再试带 padding
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(payload))
        .ok()?;

    serde_json::from_slice::<Value>(&bytes).ok().filter(|v| v.is_object())
}

fn non_empty_str(claims: &Value, key: &str) -> Option<String> {
    claims
        .get(key)
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn looks_like_email(s: &str) -> bool {
    s.contains('@')
}

/// 从载荷中提取邮箱与展示名。
///
/// - 邮箱：`email` 声明优先，缺失时取第一个含 `@` 的 preferred_username /
///   unique_name；仍无则视为拿不到；
/// - 展示名（用户名）回退链：`name` → `preferred_username` → `unique_name`
///   （Microsoft Entra ID 系 IdP 常用）→ `given_name + " " + family_name`。
pub fn extract_identity(claims: &Value) -> TokenIdentity {
    let usernameish = non_empty_str(claims, "preferred_username")
        .or_else(|| non_empty_str(claims, "unique_name"));
    let email = non_empty_str(claims, "email")
        .filter(|s| looks_like_email(s))
        .or_else(|| usernameish.clone().filter(|s| looks_like_email(s)));
    let display_name = non_empty_str(claims, "name")
        .or_else(|| usernameish)
        .or_else(|| {
            let given = claims.get("given_name").and_then(|v| v.as_str()).unwrap_or("");
            let family = claims.get("family_name").and_then(|v| v.as_str()).unwrap_or("");
            let full = format!("{} {}", given.trim(), family.trim()).trim().to_string();
            if full.is_empty() { None } else { Some(full) }
        });
    TokenIdentity { email, display_name }
}

/// 便捷封装：直接从 JWT 字符串提取身份；失败返回空结果
pub fn identity_from_jwt(token: &str) -> TokenIdentity {
    decode_jwt_payload(token)
        .as_ref()
        .map(extract_identity)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn b64url(data: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(data)
    }

    fn make_jwt(payload: &Value) -> String {
        let header = b64url(br#"{"alg":"none"}"#);
        format!("{}.{}.x", header, b64url(payload.to_string().as_bytes()))
    }

    #[test]
    fn test_decode_jwt_payload_ok() {
        let jwt = make_jwt(&json!({"email": "a@b.c", "name": "Alice"}));
        let claims = decode_jwt_payload(&jwt).unwrap();
        assert_eq!(claims["email"], "a@b.c");
    }

    #[test]
    fn test_decode_jwt_payload_rejects_garbage() {
        assert!(decode_jwt_payload("").is_none());
        assert!(decode_jwt_payload("not-a-jwt").is_none());
        assert!(decode_jwt_payload("a.b").is_none());
        // 第二段非 base64 / 非 JSON
        assert!(decode_jwt_payload("h.@@@.s").is_none());
        assert!(decode_jwt_payload(&make_jwt(&json!([1, 2]))).is_none());
    }

    #[test]
    fn test_extract_identity_full_claims() {
        let t = extract_identity(&json!({
            "email": "user@example.com",
            "name": "Alice Wang",
            "preferred_username": "alice"
        }));
        assert_eq!(t.email.as_deref(), Some("user@example.com"));
        assert_eq!(t.display_name.as_deref(), Some("Alice Wang"));
    }

    #[test]
    fn test_extract_identity_entra_unique_name_is_email() {
        // Entra ID 风格：unique_name 直接是邮箱形态，应同时充当 email 与展示名
        let t = extract_identity(&json!({ "unique_name": "bob@corp.com" }));
        assert_eq!(t.email.as_deref(), Some("bob@corp.com"));
        assert_eq!(t.display_name.as_deref(), Some("bob@corp.com"));
    }

    #[test]
    fn test_extract_identity_email_claim_not_email_shaped_ignored() {
        // email 声明若不是邮箱形态（异常数据），回落到含 @ 的用户名型声明
        let t = extract_identity(&json!({
            "email": "not-an-email",
            "preferred_username": "carol@corp.com"
        }));
        assert_eq!(t.email.as_deref(), Some("carol@corp.com"));
    }

    #[test]
    fn test_extract_identity_given_family_only() {
        let t = extract_identity(&json!({ "given_name": "Carol", "family_name": "Li" }));
        assert_eq!(t.display_name.as_deref(), Some("Carol Li"));
        assert_eq!(t.email, None);
    }

    #[test]
    fn test_extract_identity_blank_values_skipped() {
        let t = extract_identity(&json!({ "email": "", "name": "   " }));
        assert_eq!(t.email, None);
        assert_eq!(t.display_name, None);
    }

    #[test]
    fn test_identity_from_jwt_end_to_end() {
        let jwt = make_jwt(&json!({"email": "e@f.g", "preferred_username": "eve"}));
        let t = identity_from_jwt(&jwt);
        assert_eq!(t.email.as_deref(), Some("e@f.g"));
        assert_eq!(t.display_name.as_deref(), Some("eve"));
        // 非 JWT 输入返回空结果而非 panic
        assert_eq!(identity_from_jwt("garbage"), TokenIdentity::default());
    }
}

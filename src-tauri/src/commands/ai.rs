//! AI 命令：OpenAI 兼容接口（含各类中转站）与 Anthropic 官方接口。
//! 仅在用户显式配置 API Key 后可用；API 请求由 Rust 发出，不通过浏览器发送。

use crate::config;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use tauri::AppHandle;

/// 统一对话入口：system + user → 返回模型文本回复
#[tauri::command]
pub async fn ai_chat(
    app: AppHandle,
    prompt: String,
    system: Option<String>,
) -> Result<String, String> {
    let settings = config::load_settings(&app);
    if settings.ai_api_key.trim().is_empty() {
        return Err("未配置 AI API Key，请到 设置 → AI 中填写".into());
    }
    let prompt = truncate(prompt, 60_000);
    tauri::async_runtime::spawn_blocking(move || {
        chat_blocking(&settings, &prompt, system.as_deref())
    })
    .await
    .map_err(|e| format!("AI 线程异常：{}", e))?
}

/// 使用设置表单的当前值查询模型，不读取或保存持久化设置。
#[tauri::command]
pub async fn ai_list_models(
    provider: String,
    base_url: String,
    api_key: String,
) -> Result<Vec<String>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        list_models_blocking(&provider, &base_url, &api_key)
    })
    .await
    .map_err(|_| "获取模型列表线程异常".to_string())?
}

const MODEL_BODY_LIMIT: u64 = 2 * 1024 * 1024;
const MODEL_PAGE_LIMIT: usize = 20;

fn models_url(base_url: &str) -> Result<url::Url, String> {
    let base = base_url.trim().trim_end_matches('/');
    let invalid = || "AI Base URL 必须是无凭据、查询参数和片段的 HTTP(S) 地址".to_string();
    let url = url::Url::parse(base).map_err(|_| invalid())?;
    let authority = base.split_once("://").ok_or_else(invalid)?.1;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || authority
            .split(['/', '?', '#'])
            .next()
            .unwrap_or("")
            .contains('@')
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid());
    }
    let suffix = if base.ends_with("/v1") {
        "/models"
    } else {
        "/v1/models"
    };
    url::Url::parse(&format!("{base}{suffix}")).map_err(|_| invalid())
}

fn list_models_blocking(
    provider: &str,
    base_url: &str,
    api_key: &str,
) -> Result<Vec<String>, String> {
    use std::collections::BTreeSet;
    use std::io::Read;
    use std::time::{Duration, Instant};

    let endpoint = models_url(base_url)?;
    if api_key.trim().is_empty() {
        return Err("请先填写 AI API Key".into());
    }
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "HTTP 客户端创建失败".to_string())?;
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut ids = BTreeSet::new();
    let mut cursors = BTreeSet::new();
    let mut after_id: Option<String> = None;
    for _ in 0..MODEL_PAGE_LIMIT {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("获取模型列表超时".into());
        }
        let mut request = client
            .get(endpoint.clone())
            .timeout(remaining.min(Duration::from_secs(20)));
        if provider == "anthropic" {
            request = request
                .header("x-api-key", api_key)
                .header("anthropic-version", "2023-06-01");
            if let Some(cursor) = &after_id {
                request = request.query(&[("after_id", cursor)]);
            }
        } else {
            request = request.bearer_auth(api_key);
        }
        // 不展示 reqwest/JSON 错误：其中可能包含 URL、密钥或服务端响应。
        let response = request
            .send()
            .map_err(|_| "获取模型列表请求失败，请检查地址、网络和 API Key".to_string())?;
        if !response.status().is_success() {
            return Err(format!("AI 服务返回 HTTP {}", response.status().as_u16()));
        }
        if response
            .content_length()
            .is_some_and(|n| n > MODEL_BODY_LIMIT)
        {
            return Err("模型列表响应过大".into());
        }
        let mut body = Vec::new();
        response
            .take(MODEL_BODY_LIMIT + 1)
            .read_to_end(&mut body)
            .map_err(|_| "读取模型列表失败".to_string())?;
        if body.len() as u64 > MODEL_BODY_LIMIT {
            return Err("模型列表响应过大".into());
        }
        let value: Value =
            serde_json::from_slice(&body).map_err(|_| "模型列表响应不是有效 JSON".to_string())?;
        let data = value
            .get("data")
            .and_then(Value::as_array)
            .ok_or("模型列表响应缺少 data 数组")?;
        for model in data {
            let id = model
                .get("id")
                .and_then(Value::as_str)
                .filter(|id| !id.trim().is_empty())
                .ok_or("模型列表包含无效模型 ID")?;
            ids.insert(id.to_string());
        }
        let has_more = if provider == "anthropic" {
            match value.get("has_more") {
                None => false,
                Some(v) => v.as_bool().ok_or("模型列表分页标记无效")?,
            }
        } else {
            false
        };
        if !has_more {
            return Ok(ids.into_iter().collect());
        }
        let cursor = value
            .get("last_id")
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .ok_or("模型列表分页缺少 last_id")?;
        if !cursors.insert(cursor.to_string()) {
            return Err("模型列表分页游标重复".into());
        }
        after_id = Some(cursor.to_string());
    }
    Err("模型列表超过分页上限".into())
}

fn truncate(s: String, max: usize) -> String {
    if s.chars().count() <= max {
        return s;
    }
    let t: String = s.chars().take(max).collect();
    format!("{}\n…（内容过长已截断）", t)
}

fn chat_blocking(
    settings: &config::AppSettings,
    prompt: &str,
    system: Option<&str>,
) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP 客户端创建失败：{}", e))?;

    let base = settings
        .ai_base_url
        .trim()
        .trim_end_matches('/')
        .to_string();
    if settings.ai_provider == "anthropic" {
        // Anthropic Messages API（base 缺 /v1 时自动补）
        let url = if base.ends_with("/v1") {
            format!("{}/messages", base)
        } else {
            format!("{}/v1/messages", base)
        };
        let mut body = json!({
            "model": settings.ai_model,
            "max_tokens": 4096,
            "messages": [{ "role": "user", "content": prompt }],
        });
        if let Some(sys) = system {
            body["system"] = json!(sys);
        }
        let resp = client
            .post(&url)
            .header("x-api-key", &settings.ai_api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .map_err(|e| format!("请求失败：{}", e))?;
        to_text(resp, |v| {
            v.get("content")?
                .as_array()?
                .iter()
                .filter_map(|p| {
                    if p.get("type").and_then(|t| t.as_str()) == Some("text") {
                        p.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("")
                .into()
        })
    } else {
        // OpenAI 兼容 chat/completions（各类兼容中转站同构）
        let url = if base.ends_with("/v1") {
            format!("{}/chat/completions", base)
        } else {
            format!("{}/v1/chat/completions", base)
        };
        let mut messages = vec![];
        if let Some(sys) = system {
            messages.push(json!({ "role": "system", "content": sys }));
        }
        messages.push(json!({ "role": "user", "content": prompt }));
        let body = json!({ "model": settings.ai_model, "messages": messages, "stream": false });
        let resp = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", settings.ai_api_key))
            .json(&body)
            .send()
            .map_err(|e| format!("请求失败：{}", e))?;
        to_text(resp, |v| {
            v.get("choices")?
                .get(0)?
                .get("message")?
                .get("content")?
                .as_str()?
                .to_string()
                .into()
        })
    }
}

/// 解析响应：先看 HTTP 错误体，再按各家结构取文本
fn to_text(
    resp: reqwest::blocking::Response,
    pick: impl Fn(&Value) -> Option<String>,
) -> Result<String, String> {
    let status = resp.status();
    let text = resp.text().map_err(|e| format!("读取响应失败：{}", e))?;
    let v: Value = serde_json::from_str(&text)
        .map_err(|_| anyhow!("响应不是 JSON：{}", &text[..text.len().min(300)]))
        .map_err(|e| e.to_string())?;
    if !status.is_success() {
        let msg = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .or_else(|| v.get("message").and_then(|m| m.as_str()))
            .unwrap_or("未知错误");
        return Err(format!("AI 服务返回 {}：{}", status.as_u16(), msg));
    }
    pick(&v).ok_or_else(|| format!("无法解析 AI 响应：{}", &text[..text.len().min(300)]))
}

#[cfg(test)]
mod model_tests {
    use super::*;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    // Real loopback HTTP verifies reqwest headers, URL encoding, and redirect behavior.
    fn mock(responses: Vec<(u16, String)>) -> (String, JoinHandle<Vec<(String, Vec<String>)>>) {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let base = format!("http://{}", server.server_addr());
        let handle = thread::spawn(move || {
            let mut requests = vec![];
            for (status, body) in responses {
                let request = server
                    .recv_timeout(Duration::from_secs(5))
                    .unwrap()
                    .expect("expected HTTP request");
                assert_eq!(request.method(), &tiny_http::Method::Get);
                requests.push((
                    request.url().to_string(),
                    request
                        .headers()
                        .iter()
                        .map(|h| h.to_string().to_ascii_lowercase())
                        .collect(),
                ));
                let response = tiny_http::Response::from_string(body)
                    .with_status_code(status)
                    .with_header(
                        tiny_http::Header::from_bytes("Location", "/redirected-secret").unwrap(),
                    );
                // Oversize responses can be rejected before reading their body.
                let _ = request.respond(response);
            }
            requests
        });
        (base, handle)
    }

    #[test]
    fn openai_paths_headers_and_sorted_unique_ids() {
        for suffix in ["", "/v1", "/proxy/", "/proxy/v1///"] {
            let (base, server) = mock(vec![(
                200,
                r#"{"data":[{"id":"z"},{"id":"a"},{"id":"z"}]}"#.into(),
            )]);
            assert_eq!(
                list_models_blocking("openai", &format!(" {base}{suffix} "), "form-secret")
                    .unwrap(),
                vec!["a", "z"]
            );
            let requests = server.join().unwrap();
            let prefix = if suffix.starts_with("/proxy") {
                "/proxy"
            } else {
                ""
            };
            assert_eq!(requests[0].0, format!("{prefix}/v1/models"));
            assert!(requests[0]
                .1
                .contains(&"authorization: bearer form-secret".into()));
            assert!(!requests[0].1.iter().any(|h| h.starts_with("x-api-key:")));
        }
    }

    #[test]
    fn anthropic_paginates_with_encoded_cursor_and_headers() {
        let (base, server) = mock(vec![
            (
                200,
                r#"{"data":[{"id":"z"}],"has_more":true,"last_id":"cursor /?&"}"#.into(),
            ),
            (
                200,
                r#"{"data":[{"id":"a"},{"id":"z"}],"has_more":false}"#.into(),
            ),
        ]);
        assert_eq!(
            list_models_blocking("anthropic", &base, "form-secret").unwrap(),
            vec!["a", "z"]
        );
        let requests = server.join().unwrap();
        assert_eq!(requests[0].0, "/v1/models");
        assert_eq!(requests[1].0, "/v1/models?after_id=cursor+%2F%3F%26");
        for (_, headers) in requests {
            assert!(headers.contains(&"x-api-key: form-secret".into()));
            assert!(headers.contains(&"anthropic-version: 2023-06-01".into()));
            assert!(!headers.iter().any(|h| h.starts_with("authorization:")));
        }
    }

    #[test]
    fn errors_are_sanitized_and_redirects_are_not_followed() {
        for (status, body, expected) in [
            (401, "response-secret", "HTTP 401"),
            (302, "response-secret", "HTTP 302"),
            (200, "response-secret", "有效 JSON"),
            (200, r#"{"error":"response-secret"}"#, "data 数组"),
            (
                200,
                r#"{"data":[{"id":42}],"secret":"response-secret"}"#,
                "无效模型 ID",
            ),
        ] {
            let (base, server) = mock(vec![(status, body.into())]);
            let error = list_models_blocking("openai", &base, "form-secret").unwrap_err();
            assert!(error.contains(expected), "{error}");
            assert!(!error.contains("secret"));
            assert!(!error.contains(&base));
            assert_eq!(server.join().unwrap().len(), 1);
        }
        let (base, server) = mock(vec![(200, "x".repeat(MODEL_BODY_LIMIT as usize + 1))]);
        assert_eq!(
            list_models_blocking("openai", &base, "key").unwrap_err(),
            "模型列表响应过大"
        );
        server.join().unwrap();
    }

    #[test]
    fn validates_urls_and_credentials_without_echoing_them() {
        for base in [
            "",
            "ftp://example.com",
            "http://user:secret@example.com",
            "http://@example.com",
            "https://example.com?key=secret",
            "https://example.com/#secret",
            "not a url",
        ] {
            let error = list_models_blocking("openai", base, "key").unwrap_err();
            assert!(error.contains("Base URL"));
            assert!(!error.contains("secret"));
        }
        assert_eq!(
            list_models_blocking("openai", "https://example.com", "  ").unwrap_err(),
            "请先填写 AI API Key"
        );
        let error = list_models_blocking("openai", "http://127.0.0.1:1/secret", "secret\ninvalid")
            .unwrap_err();
        assert!(!error.contains("secret"));
    }

    #[test]
    fn anthropic_rejects_missing_repeated_and_excessive_cursors() {
        for body in [
            r#"{"data":[],"has_more":true}"#,
            r#"{"data":[],"has_more":"true"}"#,
        ] {
            let (base, server) = mock(vec![(200, body.into())]);
            assert!(list_models_blocking("anthropic", &base, "key")
                .unwrap_err()
                .contains("分页"));
            server.join().unwrap();
        }
        let page = r#"{"data":[],"has_more":true,"last_id":"same"}"#.to_string();
        let (base, server) = mock(vec![(200, page.clone()), (200, page)]);
        assert_eq!(
            list_models_blocking("anthropic", &base, "key").unwrap_err(),
            "模型列表分页游标重复"
        );
        assert_eq!(server.join().unwrap().len(), 2);
        let (base, server) = mock(
            (0..MODEL_PAGE_LIMIT)
                .map(|n| {
                    (
                        200,
                        json!({"data": [], "has_more": true, "last_id": n.to_string()}).to_string(),
                    )
                })
                .collect(),
        );
        assert_eq!(
            list_models_blocking("anthropic", &base, "key").unwrap_err(),
            "模型列表超过分页上限"
        );
        assert_eq!(server.join().unwrap().len(), MODEL_PAGE_LIMIT);
    }
}

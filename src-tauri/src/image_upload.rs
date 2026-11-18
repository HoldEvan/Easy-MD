//! Global image upload configuration and transports. Secrets never enter JSON or errors.
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Write},
    path::Path,
    sync::Mutex,
    time::Duration,
};
use url::Url;

const MAX_BYTES: usize = 20 * 1024 * 1024;
const SERVICE: &str = "EasyMD.image-upload";
static CONFIG_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    #[default]
    Local,
    S3,
    Picgo,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct S3Config {
    pub endpoint: String,
    pub region: String,
    pub bucket: String,
    pub access_key: String,
    pub public_base: String,
    pub prefix: String,
    pub path_style: bool,
}
impl Default for S3Config {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            region: "us-east-1".into(),
            bucket: String::new(),
            access_key: String::new(),
            public_base: String::new(),
            prefix: String::new(),
            path_style: true,
        }
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct PicgoConfig {
    pub url: String,
}
impl Default for PicgoConfig {
    fn default() -> Self {
        Self {
            url: "http://127.0.0.1:36677/upload".into(),
        }
    }
}
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct ImageUploadConfig {
    pub provider: Provider,
    pub s3: S3Config,
    pub picgo: PicgoConfig,
    #[serde(skip_deserializing)]
    pub has_secret: bool,
}
#[derive(Serialize, Deserialize, Default)]
struct StoredConfig {
    config: ImageUploadConfig,
    secret_id: Option<String>,
}
#[derive(Serialize)]
pub struct UploadResult {
    pub url: String,
}
#[derive(Serialize)]
pub struct TestResult {
    pub url: Option<String>,
    pub upload_ok: bool,
    pub accessible: Option<bool>,
    pub cleanup_ok: Option<bool>,
    pub message: String,
}

fn entry(id: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, id).map_err(|_| "系统凭据库不可用，未使用明文存储".into())
}
fn read_secret(id: Option<&str>) -> Result<Option<String>, String> {
    let Some(id) = id else { return Ok(None) };
    match entry(id)?.get_password() {
        Ok(s) => Ok(Some(s)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(_) => Err("无法读取系统凭据库，请检查系统授权".into()),
    }
}
fn load(path: &Path) -> Result<StoredConfig, String> {
    match std::fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| "图床配置文件损坏".into()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(StoredConfig::default()),
        Err(_) => Err("无法读取图床配置".into()),
    }
}
fn atomic_write(path: &Path, data: &StoredConfig) -> Result<(), String> {
    let dir = path.parent().ok_or("配置路径无效")?;
    std::fs::create_dir_all(dir).map_err(|_| "无法创建配置目录")?;
    let mut file = tempfile::NamedTempFile::new_in(dir).map_err(|_| "无法创建配置临时文件")?;
    serde_json::to_writer_pretty(&mut file, data).map_err(|_| "无法写入图床配置")?;
    file.as_file().sync_all().map_err(|_| "无法同步图床配置")?;
    file.persist(path).map_err(|_| "无法替换图床配置")?;
    Ok(())
}
fn loopback(url: &Url) -> bool {
    match url.host() {
        Some(url::Host::Domain(h)) => h.eq_ignore_ascii_case("localhost"),
        Some(url::Host::Ipv4(ip)) => ip.is_loopback(),
        Some(url::Host::Ipv6(ip)) => ip.is_loopback(),
        None => false,
    }
}
fn safe_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|_| "URL 格式无效")?;
    if url.host().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https" || (url.scheme() == "http" && loopback(&url)))
    {
        return Err("URL 必须使用 HTTPS（仅本机服务可用 HTTP），且不能包含凭据或片段".into());
    }
    Ok(url)
}
fn validate(config: &ImageUploadConfig) -> Result<(), String> {
    match config.provider {
        Provider::Local => Ok(()),
        Provider::Picgo => {
            let u = safe_url(&config.picgo.url)?;
            if !loopback(&u) || u.query().is_some() {
                return Err("PicGo 本地路径协议仅允许回环地址，且不能包含查询参数".into());
            }
            Ok(())
        }
        Provider::S3 => {
            let s = &config.s3;
            let u = safe_url(&s.endpoint)?;
            if u.query().is_some() {
                return Err("S3 endpoint 不能包含查询参数".into());
            }
            if s.region.is_empty()
                || !s
                    .region
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
                || s.access_key.is_empty()
                || s.access_key.chars().any(char::is_whitespace)
                || s.access_key.contains('/')
                || s.bucket.is_empty()
                || !s
                    .bucket
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
                || s.bucket.starts_with('.')
                || s.bucket.ends_with('.')
                || s.bucket.contains("..")
            {
                return Err("请填写有效的 S3 region、bucket 和 access key".into());
            }
            if !s.public_base.is_empty() && safe_url(&s.public_base)?.query().is_some() {
                return Err("公开访问地址不能包含查询参数".into());
            }
            if s.prefix.contains('\\')
                || s.prefix.chars().any(char::is_control)
                || s.prefix.split('/').any(|p| p == "." || p == "..")
            {
                return Err("对象前缀无效".into());
            }
            Ok(())
        }
    }
}
fn config_path(app: &tauri::AppHandle) -> std::path::PathBuf {
    crate::config::config_dir(app).join("image-upload.json")
}
#[tauri::command]
pub async fn image_upload_get_config(app: tauri::AppHandle) -> Result<ImageUploadConfig, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = CONFIG_LOCK.lock().map_err(|_| "配置锁不可用")?;
        let mut stored = load(&config_path(&app))?;
        stored.config.has_secret = read_secret(stored.secret_id.as_deref())?.is_some();
        Ok(stored.config)
    })
    .await
    .map_err(|_| "读取图床配置任务失败")?
}
#[tauri::command]
pub async fn image_upload_save_config(
    app: tauri::AppHandle,
    mut config: ImageUploadConfig,
    secret: Option<String>,
) -> Result<ImageUploadConfig, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = CONFIG_LOCK.lock().map_err(|_| "配置锁不可用")?;
        validate(&config)?;
        let path = config_path(&app);
        let previous = load(&path)?;
        let mut secret_id = previous.secret_id.clone();
        if let Some(value) = secret.as_ref() {
            if value.is_empty() {
                secret_id = None;
            } else {
                if value.len() > 8192 {
                    return Err("密钥过长".into());
                }
                let id = uuid::Uuid::new_v4().to_string();
                entry(&id)?
                    .set_password(value)
                    .map_err(|_| "无法保存到系统凭据库，未使用明文存储")?;
                secret_id = Some(id);
            }
        }
        config.has_secret = match read_secret(secret_id.as_deref()) {
            Ok(value) if secret.as_ref().is_some_and(|s| !s.is_empty()) && value.is_none() => {
                if let Some(id) = secret_id.as_deref() {
                    let _ = entry(id)
                        .and_then(|e| e.delete_credential().map_err(|_| "凭据清理失败".into()));
                }
                return Err("系统凭据库未保留密钥，配置未保存".into());
            }
            Ok(value) => value.is_some(),
            Err(error) => {
                if secret_id != previous.secret_id {
                    if let Some(id) = secret_id.as_deref() {
                        let _ = entry(id)
                            .and_then(|e| e.delete_credential().map_err(|_| "凭据清理失败".into()));
                    }
                }
                return Err(error);
            }
        };
        let stored = StoredConfig {
            config: config.clone(),
            secret_id: secret_id.clone(),
        };
        if let Err(e) = atomic_write(&path, &stored) {
            if secret_id != previous.secret_id {
                if let Some(id) = secret_id {
                    let _ = entry(&id)
                        .and_then(|e| e.delete_credential().map_err(|_| "凭据清理失败".into()));
                }
            }
            return Err(e);
        }
        if secret_id != previous.secret_id {
            if let Some(id) = previous.secret_id {
                let _ = entry(&id)
                    .and_then(|e| e.delete_credential().map_err(|_| "凭据清理失败".into()));
            }
        }
        Ok(config)
    })
    .await
    .map_err(|_| "保存图床配置任务失败")?
}
fn snapshot(app: &tauri::AppHandle) -> Result<(ImageUploadConfig, Option<String>), String> {
    let _lock = CONFIG_LOCK.lock().map_err(|_| "配置锁不可用")?;
    let stored = load(&config_path(app))?;
    validate(&stored.config)?;
    let secret = if stored.config.provider == Provider::S3 {
        read_secret(stored.secret_id.as_deref())?
    } else {
        None
    };
    Ok((stored.config, secret))
}
fn client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(10))
        .redirect(reqwest::redirect::Policy::none())
        .no_proxy()
        .build()
        .map_err(|_| "无法创建上传客户端".into())
}
fn validate_image(bytes: &[u8], ext: &str) -> Result<(&'static str, &'static str), String> {
    if bytes.is_empty() || bytes.len() > MAX_BYTES {
        return Err("图片必须为 1 字节至 20 MiB".into());
    }
    let (format, extension, mime) = match ext.to_ascii_lowercase().trim_start_matches('.') {
        "png" => (image::ImageFormat::Png, "png", "image/png"),
        "jpg" | "jpeg" => (image::ImageFormat::Jpeg, "jpg", "image/jpeg"),
        "gif" => (image::ImageFormat::Gif, "gif", "image/gif"),
        "webp" => (image::ImageFormat::WebP, "webp", "image/webp"),
        "bmp" => (image::ImageFormat::Bmp, "bmp", "image/bmp"),
        "ico" => (image::ImageFormat::Ico, "ico", "image/x-icon"),
        _ => return Err("图床支持 PNG、JPEG、GIF、WebP、BMP、ICO 图片".into()),
    };
    if image::guess_format(bytes).ok() != Some(format) {
        return Err("图片内容与扩展名不一致".into());
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(16384);
    limits.max_image_height = Some(16384);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    reader.decode().map_err(|_| "图片损坏或尺寸超过限制")?;
    Ok((extension, mime))
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn hmac(key: &[u8], data: &str) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC accepts any key size");
    mac.update(data.as_bytes());
    mac.finalize().into_bytes().to_vec()
}
fn append_path(mut url: Url, segments: &[&str]) -> Result<Url, String> {
    {
        let mut path = url.path_segments_mut().map_err(|_| "URL 不支持路径")?;
        path.pop_if_empty();
        for segment in segments {
            path.push(segment);
        }
    }
    // SigV4 URI encoding is stricter than WHATWG URL path encoding.
    let canonical = url
        .path()
        .split('/')
        .map(|part| {
            let decoded = percent_encoding::percent_decode_str(part).collect::<Vec<_>>();
            decoded
                .iter()
                .map(|b| {
                    if b.is_ascii_alphanumeric() || b"-_.~".contains(b) {
                        (*b as char).to_string()
                    } else {
                        format!("%{b:02X}")
                    }
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/");
    url.set_path(&canonical);
    Ok(url)
}
fn object_url(s: &S3Config, key: &str) -> Result<Url, String> {
    let mut url = safe_url(&s.endpoint)?;
    let mut segments = Vec::new();
    if s.path_style {
        segments.push(s.bucket.as_str());
    } else {
        let host = url.host_str().ok_or("S3 endpoint 缺少主机")?;
        let host = format!("{}.{}", s.bucket, host);
        url.set_host(Some(&host))
            .map_err(|_| "虚拟主机 bucket 地址无效")?;
    }
    segments.extend(key.split('/'));
    append_path(url, &segments)
}
fn public_url(s: &S3Config, key: &str) -> Result<Url, String> {
    if s.public_base.is_empty() {
        object_url(s, key)
    } else {
        append_path(
            safe_url(&s.public_base)?,
            &key.split('/').collect::<Vec<_>>(),
        )
    }
}
fn signature(
    s: &S3Config,
    secret: &str,
    method: &str,
    url: &Url,
    bytes: &[u8],
    date: &str,
) -> (String, String) {
    let payload = hex(&Sha256::digest(bytes));
    let host = match url.port() {
        Some(port) => format!("{}:{port}", url.host_str().unwrap_or("")),
        None => url.host_str().unwrap_or("").to_string(),
    };
    let signed = "host;x-amz-content-sha256;x-amz-date";
    let canonical = format!("{method}\n{}\n\nhost:{host}\nx-amz-content-sha256:{payload}\nx-amz-date:{date}\n\n{signed}\n{payload}", url.path());
    let day = &date[..8];
    let scope = format!("{day}/{}/s3/aws4_request", s.region);
    let to_sign = format!(
        "AWS4-HMAC-SHA256\n{date}\n{scope}\n{}",
        hex(&Sha256::digest(canonical.as_bytes()))
    );
    let key = hmac(format!("AWS4{secret}").as_bytes(), day);
    let key = hmac(&key, &s.region);
    let key = hmac(&key, "s3");
    let key = hmac(&key, "aws4_request");
    (
        format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed}, Signature={}",
            s.access_key,
            hex(&hmac(&key, &to_sign))
        ),
        payload,
    )
}
fn s3_request(
    client: &Client,
    s: &S3Config,
    secret: &str,
    method: reqwest::Method,
    url: &Url,
    bytes: &[u8],
    mime: &str,
) -> Result<(), String> {
    let date = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let (auth, payload) = signature(s, secret, method.as_str(), url, bytes, &date);
    let response = client
        .request(method, url.clone())
        .header("Authorization", auth)
        .header("x-amz-date", date)
        .header("x-amz-content-sha256", payload)
        .header("Content-Type", mime)
        .body(bytes.to_vec())
        .send()
        .map_err(|_| "S3 请求失败，请检查网络、endpoint 和证书")?;
    if !response.status().is_success() {
        return Err(format!(
            "S3 请求失败（HTTP {}），请检查权限和配置",
            response.status().as_u16()
        ));
    }
    Ok(())
}
struct Uploaded {
    url: String,
    s3_object: Option<Url>,
}
fn upload(
    client: &Client,
    config: &ImageUploadConfig,
    secret: Option<&str>,
    bytes: &[u8],
    ext: &str,
) -> Result<Uploaded, String> {
    validate(config)?;
    let (ext, mime) = validate_image(bytes, ext)?;
    match config.provider {
        Provider::Local => Err("本地图片请使用现有附件保存功能".into()),
        Provider::S3 => {
            let secret = secret
                .filter(|s| !s.is_empty())
                .ok_or("请先在系统凭据库保存 S3 secret key")?;
            let prefix = config.s3.prefix.trim_matches('/');
            let name = format!("{}.{}", uuid::Uuid::new_v4(), ext);
            let key = if prefix.is_empty() {
                name
            } else {
                format!("{prefix}/{name}")
            };
            let object = object_url(&config.s3, &key)?;
            let public = public_url(&config.s3, &key)?;
            s3_request(
                client,
                &config.s3,
                secret,
                reqwest::Method::PUT,
                &object,
                bytes,
                mime,
            )?;
            Ok(Uploaded {
                url: public.into(),
                s3_object: Some(object),
            })
        }
        Provider::Picgo => {
            // RAII removes the private temporary file on success and every error path.
            let mut temp = tempfile::Builder::new()
                .prefix("easymd-upload-")
                .suffix(&format!(".{ext}"))
                .tempfile()
                .map_err(|_| "无法创建图片临时文件")?;
            temp.write_all(bytes).map_err(|_| "无法写入图片临时文件")?;
            temp.flush().map_err(|_| "无法刷新图片临时文件")?;
            let path = temp.path().to_str().ok_or("临时图片路径不是有效 UTF-8")?;
            let response = client
                .post(&config.picgo.url)
                .json(&serde_json::json!({"list": [path]}))
                .send()
                .map_err(|_| "无法连接本机 PicGo，请确认服务已启动")?;
            if !response.status().is_success() {
                return Err(format!(
                    "PicGo 上传失败（HTTP {}）",
                    response.status().as_u16()
                ));
            }
            let mut body = Vec::new();
            response
                .take(1024 * 1024 + 1)
                .read_to_end(&mut body)
                .map_err(|_| "无法读取 PicGo 响应")?;
            if body.len() > 1024 * 1024 {
                return Err("PicGo 响应过大".into());
            }
            let value: serde_json::Value =
                serde_json::from_slice(&body).map_err(|_| "PicGo 返回无效 JSON")?;
            if value.get("success").and_then(|v| v.as_bool()) != Some(true) {
                return Err("PicGo 报告上传失败，请在 PicGo 中检查图床配置".into());
            }
            let raw = value
                .get("result")
                .and_then(|v| v.as_array())
                .and_then(|v| v.first())
                .and_then(|v| v.as_str())
                .ok_or("PicGo 未返回图片 URL")?;
            let url = safe_url(raw)?.to_string();
            Ok(Uploaded {
                url,
                s3_object: None,
            })
        }
    }
}
#[tauri::command]
pub async fn image_upload(
    app: tauri::AppHandle,
    bytes: Vec<u8>,
    ext: String,
) -> Result<UploadResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (config, secret) = snapshot(&app)?;
        let uploaded = upload(&client()?, &config, secret.as_deref(), &bytes, &ext)?;
        Ok(UploadResult { url: uploaded.url })
    })
    .await
    .map_err(|_| "图片上传任务失败")?
}
fn test_upload(client: &Client, config: &ImageUploadConfig, secret: Option<&str>) -> TestResult {
    // A real encoded 1x1 PNG exercises the same validation and transport as normal uploads.
    let mut bytes = std::io::Cursor::new(Vec::new());
    if image::DynamicImage::new_rgba8(1, 1)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .is_err()
    {
        return TestResult {
            url: None,
            upload_ok: false,
            accessible: None,
            cleanup_ok: None,
            message: "无法生成测试图片".into(),
        };
    }
    let uploaded = match upload(client, config, secret, bytes.get_ref(), "png") {
        Ok(value) => value,
        Err(message) => {
            return TestResult {
                url: None,
                upload_ok: false,
                accessible: None,
                cleanup_ok: None,
                message,
            }
        }
    };
    let accessible = client
        .get(&uploaded.url)
        .header("Range", "bytes=0-0")
        .send()
        .map(|r| {
            r.status().is_success()
                && r.headers()
                    .get(reqwest::header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.to_ascii_lowercase().starts_with("image/"))
        })
        .unwrap_or(false);
    let cleanup = uploaded.s3_object.as_ref().map(|object| {
        s3_request(
            client,
            &config.s3,
            secret.unwrap_or(""),
            reqwest::Method::DELETE,
            object,
            &[],
            "application/octet-stream",
        )
        .is_ok()
    });
    let message = match cleanup {
        Some(true) => "测试图片已上传并删除；URL 仅供检查，已不再保留",
        Some(false) => "测试图片上传成功，但删除失败；请在图床手动删除该 URL 对应对象",
        None => "测试图片上传成功；PicGo 不提供通用删除接口，请在图床手动删除该 URL 对应图片",
    }
    .to_string();
    TestResult {
        url: Some(uploaded.url),
        upload_ok: true,
        accessible: Some(accessible),
        cleanup_ok: cleanup,
        message,
    }
}
#[tauri::command]
pub async fn image_upload_test(app: tauri::AppHandle) -> Result<TestResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let (config, secret) = snapshot(&app)?;
        Ok(test_upload(&client()?, &config, secret.as_deref()))
    })
    .await
    .map_err(|_| "图床测试任务失败")?
}

#[cfg(test)]
mod tests {
    use super::*;
    fn png() -> Vec<u8> {
        let mut bytes = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(1, 1)
            .write_to(&mut bytes, image::ImageFormat::Png)
            .unwrap();
        bytes.into_inner()
    }
    fn s3(endpoint: String) -> ImageUploadConfig {
        ImageUploadConfig {
            provider: Provider::S3,
            s3: S3Config {
                endpoint,
                bucket: "test-bucket".into(),
                access_key: "test-key".into(),
                ..Default::default()
            },
            ..Default::default()
        }
    }
    #[test]
    fn validation_rejects_unsafe_urls_and_images() {
        for value in [
            "http://example.com",
            "https://user:secret@example.com",
            "file:///tmp/a",
            "https://example.com/#fragment",
        ] {
            assert!(safe_url(value).is_err());
        }
        assert!(safe_url("http://127.0.0.1:9000").is_ok());
        assert!(safe_url("http://[::1]:9000").is_ok());
        let mut config = ImageUploadConfig {
            provider: Provider::Picgo,
            ..Default::default()
        };
        assert!(validate(&config).is_ok());
        config.picgo.url = "https://example.com/upload".into();
        assert!(validate(&config).is_err());
        assert!(validate_image(&png(), "png").is_ok());
        assert!(validate_image(&png(), "jpg").is_err());
        assert!(validate_image(b"not an image", "png").is_err());
        assert!(validate_image(&[], "png").is_err());
        assert!(validate_image(&vec![0; MAX_BYTES + 1], "png").is_err());
        assert!(validate_image(&png(), "../png").is_err());
    }
    #[test]
    fn config_is_separate_atomic_and_contains_no_secret() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("image-upload.json");
        let mut stored = StoredConfig::default();
        atomic_write(&path, &stored).unwrap();
        stored.config.provider = Provider::Picgo;
        atomic_write(&path, &stored).unwrap();
        assert!(load(&path).unwrap().config.provider == Provider::Picgo);
        assert!(!std::fs::read_to_string(path).unwrap().contains("password"));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    #[test]
    fn object_paths_encode_unicode_and_preserve_prefix() {
        let mut s = s3("https://s3.example.com/base/".into()).s3;
        assert_eq!(
            object_url(&s, "图片/a b.png").unwrap().as_str(),
            "https://s3.example.com/base/test-bucket/%E5%9B%BE%E7%89%87/a%20b.png"
        );
        assert_eq!(
            object_url(&s, "!'()*%/a.png").unwrap().path(),
            "/base/test-bucket/%21%27%28%29%2A%25/a.png"
        );
        s.path_style = false;
        assert_eq!(
            object_url(&s, "a.png").unwrap().host_str(),
            Some("test-bucket.s3.example.com")
        );
        s.public_base = "https://cdn.example.com/images".into();
        assert_eq!(
            public_url(&s, "a.png").unwrap().as_str(),
            "https://cdn.example.com/images/a.png"
        );
    }
    #[test]
    fn sigv4_matches_independent_reference_fixture() {
        let s = S3Config {
            region: "us-east-1".into(),
            access_key: "AKIDEXAMPLE".into(),
            ..Default::default()
        };
        let (auth, payload) = signature(
            &s,
            "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
            "PUT",
            &Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap(),
            b"Welcome to Amazon S3.",
            "20130524T000000Z",
        );
        assert_eq!(payload, hex(&Sha256::digest(b"Welcome to Amazon S3.")));
        assert!(auth.contains("Credential=AKIDEXAMPLE/20130524/us-east-1/s3/aws4_request"));
        assert_eq!(
            auth.split("Signature=").nth(1).unwrap(),
            "6b472b2701b66e0f66f097ddef1d3fa0c15434009ae32df4ad924c0b16f0f68b"
        );
        // Deterministic and sensitive to payload, HTTP method, object path and secret.
        for (method, body, secret) in [
            ("DELETE", b"".as_slice(), "secret"),
            ("PUT", b"different".as_slice(), "secret"),
        ] {
            assert_ne!(
                auth,
                signature(
                    &s,
                    secret,
                    method,
                    &Url::parse("https://examplebucket.s3.amazonaws.com/test.txt").unwrap(),
                    body,
                    "20130524T000000Z"
                )
                .0
            );
        }
    }
    #[test]
    fn picgo_uses_existing_temporary_path_and_removes_it() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let worker = std::thread::spawn(move || {
            let mut req = server.recv().unwrap();
            assert_eq!(req.method(), &tiny_http::Method::Post);
            assert_eq!(req.url(), "/upload");
            let value: serde_json::Value = serde_json::from_reader(req.as_reader()).unwrap();
            let path = value["list"][0].as_str().unwrap().to_string();
            assert_eq!(std::fs::read(&path).unwrap(), png());
            req.respond(tiny_http::Response::from_string(
                "{\"success\":true,\"result\":[\"https://example.com/image.png\"]}",
            ))
            .unwrap();
            path
        });
        let config = ImageUploadConfig {
            provider: Provider::Picgo,
            picgo: PicgoConfig {
                url: format!("http://{addr}/upload"),
            },
            ..Default::default()
        };
        assert_eq!(
            upload(&client().unwrap(), &config, None, &png(), "png")
                .unwrap()
                .url,
            "https://example.com/image.png"
        );
        assert!(!Path::new(&worker.join().unwrap()).exists());
    }
    #[test]
    fn picgo_failure_is_redacted_and_temporary_file_removed() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let worker = std::thread::spawn(move || {
            let mut req = server.recv().unwrap();
            let value: serde_json::Value = serde_json::from_reader(req.as_reader()).unwrap();
            let path = value["list"][0].as_str().unwrap().to_string();
            req.respond(tiny_http::Response::from_string(
                "{\"success\":false,\"message\":\"secret-sensitive-server-detail\"}",
            ))
            .unwrap();
            path
        });
        let config = ImageUploadConfig {
            provider: Provider::Picgo,
            picgo: PicgoConfig {
                url: format!("http://{addr}/upload"),
            },
            ..Default::default()
        };
        let error = upload(&client().unwrap(), &config, None, &png(), "png")
            .err()
            .unwrap();
        assert!(!error.contains("secret-sensitive-server-detail"));
        assert!(!Path::new(&worker.join().unwrap()).exists());
    }
    #[test]
    fn s3_test_reports_inaccessible_image_and_failed_cleanup() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let worker = std::thread::spawn(move || {
            for status in [200, 200, 403] {
                let req = server.recv().unwrap();
                req.respond(
                    tiny_http::Response::from_string("secret-sensitive-server-detail")
                        .with_status_code(status),
                )
                .unwrap();
            }
        });
        let result = test_upload(
            &client().unwrap(),
            &s3(format!("http://{addr}")),
            Some("test-secret"),
        );
        assert!(result.upload_ok);
        assert_eq!(result.accessible, Some(false));
        assert_eq!(result.cleanup_ok, Some(false));
        assert!(!result.message.contains("secret-sensitive-server-detail"));
        worker.join().unwrap();
    }
    #[test]
    fn s3_test_put_checks_access_then_deletes_only_test_object() {
        let server = tiny_http::Server::http("127.0.0.1:0").unwrap();
        let addr = server.server_addr().to_string();
        let worker = std::thread::spawn(move || {
            let mut path = String::new();
            for method in [
                tiny_http::Method::Put,
                tiny_http::Method::Get,
                tiny_http::Method::Delete,
            ] {
                let req = server.recv().unwrap();
                assert_eq!(req.method(), &method);
                if path.is_empty() {
                    path = req.url().into();
                } else {
                    assert_eq!(path, req.url());
                }
                if method != tiny_http::Method::Get {
                    assert!(req.headers().iter().any(|h| h.field.equiv("Authorization")));
                }
                req.respond(tiny_http::Response::empty(200).with_header(
                    tiny_http::Header::from_bytes("Content-Type", "image/png").unwrap(),
                ))
                .unwrap();
            }
        });
        let result = test_upload(
            &client().unwrap(),
            &s3(format!("http://{addr}")),
            Some("test-secret"),
        );
        assert!(result.upload_ok);
        assert_eq!(result.accessible, Some(true));
        assert_eq!(result.cleanup_ok, Some(true));
        worker.join().unwrap();
    }
}

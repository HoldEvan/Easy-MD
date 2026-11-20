//! Explicitly started, read-only LAN sharing. Never follows the active vault after startup.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream},
    path::{Component, Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

// tiny_http does not expose accepted sockets, and dropping Server does not cancel
// responses already handed out. A bounded loopback relay gives us socket ownership
// without duplicating HTTP parsing. Nonblocking I/O lets cancellation stop relays
// even on Windows, where shutdown may not interrupt an already-blocked recv.
struct ShareConnection {
    client: TcpStream,
    upstream: TcpStream,
    stop: Arc<AtomicBool>,
    workers: Vec<JoinHandle<()>>,
}

fn relay_copy(
    mut source: TcpStream,
    mut destination: TcpStream,
    stop: &AtomicBool,
    read_timeout: Option<Duration>,
) -> std::io::Result<()> {
    let mut buffer = [0; 16 * 1024];
    let mut last_read = std::time::Instant::now();
    while !stop.load(Ordering::Acquire) {
        let count = match source.read(&mut buffer) {
            Ok(0) => return Ok(()),
            Ok(count) => count,
            Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                if read_timeout.is_some_and(|timeout| last_read.elapsed() >= timeout) {
                    return Err(std::io::ErrorKind::TimedOut.into());
                }
                thread::park_timeout(Duration::from_millis(2));
                continue;
            }
            Err(error) => return Err(error),
        };
        let mut written = 0;
        let mut last_write = std::time::Instant::now();
        while written < count && !stop.load(Ordering::Acquire) {
            match destination.write(&buffer[written..count]) {
                Ok(0) => return Err(std::io::ErrorKind::WriteZero.into()),
                Ok(count) => {
                    written += count;
                    last_write = std::time::Instant::now();
                }
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => continue,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if last_write.elapsed() >= Duration::from_secs(5) {
                        return Err(std::io::ErrorKind::TimedOut.into());
                    }
                    thread::park_timeout(Duration::from_millis(2));
                }
                Err(error) => return Err(error),
            }
        }
        last_read = std::time::Instant::now();
    }
    Err(std::io::ErrorKind::ConnectionAborted.into())
}
impl ShareConnection {
    fn start(client: TcpStream, address: SocketAddr) -> std::io::Result<Self> {
        let upstream = TcpStream::connect(address)?;
        client.set_nonblocking(true)?;
        upstream.set_nonblocking(true)?;
        let mut connection = Self {
            client,
            upstream,
            stop: Arc::new(AtomicBool::new(false)),
            workers: Vec::new(),
        };
        for (source, destination, read_timeout) in [
            (
                &connection.client,
                &connection.upstream,
                Some(Duration::from_secs(30)),
            ),
            (&connection.upstream, &connection.client, None),
        ] {
            let source = source.try_clone()?;
            let destination = destination.try_clone()?;
            let shutdown_source = source.try_clone()?;
            let shutdown_destination = destination.try_clone()?;
            let stop = connection.stop.clone();
            connection
                .workers
                .push(
                    thread::Builder::new()
                        .name("share-relay".into())
                        .spawn(move || {
                            if relay_copy(source, destination, &stop, read_timeout).is_ok() {
                                let _ = shutdown_destination.shutdown(Shutdown::Write);
                            } else {
                                stop.store(true, Ordering::Release);
                                let _ = shutdown_source.shutdown(Shutdown::Both);
                                let _ = shutdown_destination.shutdown(Shutdown::Both);
                            }
                        })?,
                );
        }
        Ok(connection)
    }
}
impl Drop for ShareConnection {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.client.shutdown(Shutdown::Both);
        let _ = self.upstream.shutdown(Shutdown::Both);
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}
struct CancelReader<R> {
    reader: R,
    stop: Arc<AtomicBool>,
}
impl<R: std::io::Read> std::io::Read for CancelReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if self.stop.load(Ordering::Acquire) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::ConnectionAborted,
                "Sharing stopped",
            ));
        }
        self.reader.read(buffer)
    }
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareStatus {
    running: bool,
    port: Option<u16>,
    vault_name: Option<String>,
    urls: Vec<String>,
}

#[derive(Default)]
pub struct ShareState(Mutex<Option<RunningShare>>);

struct RunningShare {
    status: ShareStatus,
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl Drop for RunningShare {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl ShareState {
    fn status(&self) -> Result<ShareStatus, String> {
        let state = self.0.lock().map_err(|_| "分享状态不可用")?;
        Ok(state.as_ref().map(|s| s.status.clone()).unwrap_or_default())
    }

    fn start(
        &self,
        root: PathBuf,
        port: u16,
        password: String,
        bind: Ipv4Addr,
    ) -> Result<ShareStatus, String> {
        if port == 0 {
            return Err("端口必须为 1–65535，默认端口为 80".into());
        }
        let mut state = self.0.lock().map_err(|_| "分享状态不可用")?;
        if state.is_some() {
            return Err("分享已经启动，请先停止再更改端口或密码".into());
        }
        let root = root.canonicalize().map_err(|_| "无法访问知识库目录")?;
        if !root.is_dir() {
            return Err("知识库目录不存在".into());
        }
        // Bind before changing state: a conflict never leaves a phantom running service.
        let listener = TcpListener::bind(SocketAddr::from((bind, port)))
            .map_err(|e| format!("无法启动分享（端口 {port}）：{e}"))?;
        listener
            .set_nonblocking(true)
            .map_err(|e| format!("无法启动分享：{e}"))?;
        let server = Server::http((Ipv4Addr::LOCALHOST, 0))
            .map_err(|e| format!("无法启动 HTTP 服务：{e}"))?;
        let upstream = server
            .server_addr()
            .to_ip()
            .ok_or("无法获取 HTTP 服务地址")?;
        let name = root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let status = ShareStatus {
            running: true,
            port: Some(port),
            vault_name: Some(name.clone()),
            urls: share_urls(port),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = stop.clone();
        let password_hash = (!password.is_empty()).then(|| {
            let mut secret = Sha256::digest(password.as_bytes()).to_vec();
            secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
            secret.extend_from_slice(uuid::Uuid::new_v4().as_bytes());
            secret
        });
        let worker = thread::Builder::new()
            .name("vault-share".into())
            .spawn(move || {
                let mut connections: Vec<ShareConnection> = Vec::new();
                let mut responses: Vec<JoinHandle<()>> = Vec::new();
                while !worker_stop.load(Ordering::Acquire) {
                    connections
                        .retain(|connection| !connection.workers.iter().all(|w| w.is_finished()));
                    let mut index = 0;
                    while index < responses.len() {
                        if responses[index].is_finished() {
                            let _ = responses.swap_remove(index).join();
                        } else {
                            index += 1;
                        }
                    }
                    // Browsers keep several idle sockets per origin; leave room for
                    // new tabs while keeping relay workers bounded (32 x 2).
                    if connections.len() < 32 {
                        if let Ok((client, _)) = listener.accept() {
                            if let Ok(connection) = ShareConnection::start(client, upstream) {
                                connections.push(connection);
                            }
                        }
                    }
                    if responses.len() >= 4 {
                        thread::park_timeout(Duration::from_millis(2));
                        continue;
                    }
                    match server.recv_timeout(Duration::from_millis(2)) {
                        Ok(Some(request)) => {
                            let (root, name, password_hash, stop) = (
                                root.clone(),
                                name.clone(),
                                password_hash.clone(),
                                worker_stop.clone(),
                            );
                            if let Ok(response) = thread::Builder::new()
                                .name("share-response".into())
                                .spawn(move || {
                                    handle_request(
                                        request,
                                        &root,
                                        &name,
                                        password_hash.as_deref(),
                                        stop,
                                    );
                                })
                            {
                                responses.push(response);
                            }
                        }
                        Ok(None) => {}
                        Err(_) => break,
                    }
                }
                // Close every relay before joining responses: this interrupts tiny_http
                // socket writes and incomplete request-body reads, not just file reads.
                drop(connections);
                for response in responses {
                    let _ = response.join();
                }
                // The externally visible listener is owned here, so join releases its port.
            })
            .map_err(|e| format!("无法启动分享线程：{e}"))?;
        *state = Some(RunningShare {
            status: status.clone(),
            stop,
            worker: Some(worker),
        });
        Ok(status)
    }

    pub fn stop(&self) -> Result<ShareStatus, String> {
        let mut state = self.0.lock().map_err(|_| "分享状态不可用")?;
        drop(state.take());
        Ok(ShareStatus::default())
    }
}

#[tauri::command]
pub fn share_status(state: tauri::State<'_, ShareState>) -> Result<ShareStatus, String> {
    state.status()
}

#[tauri::command]
pub fn share_start(
    app_state: tauri::State<'_, crate::state::AppState>,
    state: tauri::State<'_, ShareState>,
    port: u16,
    password: String,
) -> Result<ShareStatus, String> {
    let root = app_state
        .vault
        .lock()
        .map_err(|_| "知识库状态不可用")?
        .as_ref()
        .ok_or("请先打开知识库")?
        .root
        .clone();
    state.start(root, port, password, Ipv4Addr::UNSPECIFIED)
}

#[tauri::command]
pub fn share_stop(state: tauri::State<'_, ShareState>) -> Result<ShareStatus, String> {
    state.stop()
}

/// Export the currently open vault as a self-contained static site ZIP.
/// `destination` must be selected by the user through the native save dialog.
#[tauri::command]
pub fn share_export_static(
    app_state: tauri::State<'_, crate::state::AppState>,
    destination: String,
) -> Result<String, String> {
    let root = app_state
        .vault
        .lock()
        .map_err(|_| "知识库状态不可用")?
        .as_ref()
        .ok_or("请先打开知识库")?
        .root
        .clone()
        .canonicalize()
        .map_err(|_| "无法访问知识库目录")?;
    let destination = PathBuf::from(destination);
    let parent = destination
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or("导出位置无效")?
        .canonicalize()
        .map_err(|_| "无法访问导出目录")?;
    let filename = destination.file_name().ok_or("导出文件名无效")?;
    if destination.extension().and_then(|s| s.to_str()) != Some("zip") {
        return Err("导出文件必须使用 .zip 扩展名".into());
    }
    let destination = parent.join(filename);
    if destination.starts_with(&root) {
        return Err("导出文件不能保存到知识库目录内".into());
    }
    if fs::symlink_metadata(&destination).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("不能覆盖符号链接".into());
    }
    let name = root
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    write_static_archive(&root, &destination, &name)?;
    Ok(destination.to_string_lossy().into_owned())
}

fn write_static_archive(root: &Path, destination: &Path, name: &str) -> Result<(), String> {
    let catalog = catalog(root, name).map_err(|_| "无法读取知识库目录")?;
    let temp = destination.with_extension("zip.tmp");
    if fs::symlink_metadata(&temp).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err("不能覆盖临时符号链接".into());
    }
    let file = fs::File::create(&temp).map_err(|e| format!("无法创建导出文件：{e}"))?;
    let mut archive = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let static_html =
        include_str!("../../src/share/index.html").replace("<body>", "<body data-static=\"true\">");
    for (path, bytes) in [
        ("index.html", static_html.as_bytes()),
        (
            "share.js",
            include_bytes!("../../src/share/share.js").as_slice(),
        ),
        (
            "share.css",
            include_bytes!("../../src/share/share.css").as_slice(),
        ),
    ] {
        archive
            .start_file(path, options)
            .map_err(|e| format!("无法写入 ZIP：{e}"))?;
        archive
            .write_all(bytes)
            .map_err(|e| format!("无法写入 ZIP：{e}"))?;
    }
    archive
        .start_file("catalog.json", options)
        .map_err(|e| format!("无法写入 ZIP：{e}"))?;
    archive
        .write_all(&serde_json::to_vec(&catalog).map_err(|e| format!("无法序列化目录：{e}"))?)
        .map_err(|e| format!("无法写入 ZIP：{e}"))?;
    for entry in catalog
        .entries
        .iter()
        .filter(|entry| entry.kind != "folder")
    {
        // Revalidate every path at read time; never follow symlinks even if the vault changed
        // between catalog construction and archive creation.
        let relative = Path::new(&entry.path);
        if relative
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
            || relative
                .components()
                .any(|c| c.as_os_str().to_str().is_none_or(|s| !allowed_name(s)))
        {
            continue;
        }
        let path = root.join(relative);
        let metadata = fs::symlink_metadata(&path).map_err(|_| "知识库文件已变化，请重试导出")?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            continue;
        }
        let canonical = path
            .canonicalize()
            .map_err(|_| "知识库文件已变化，请重试导出")?;
        if !canonical.starts_with(&root) {
            continue;
        }
        let archive_path = format!("files/{}", entry.path.replace('\\', "/"));
        archive
            .start_file(archive_path, options)
            .map_err(|e| format!("无法写入 ZIP：{e}"))?;
        let mut input =
            fs::File::open(canonical).map_err(|e| format!("无法读取知识库文件：{e}"))?;
        std::io::copy(&mut input, &mut archive).map_err(|e| format!("无法写入 ZIP：{e}"))?;
    }
    archive.finish().map_err(|e| format!("无法完成 ZIP：{e}"))?;
    fs::rename(&temp, destination).map_err(|e| format!("无法替换导出文件：{e}"))?;
    Ok(())
}

fn share_urls(port: u16) -> Vec<String> {
    let mut urls = vec![format!("http://localhost:{port}/")];
    if let Ok(interfaces) = if_addrs::get_if_addrs() {
        for interface in interfaces {
            if let std::net::IpAddr::V4(ip) = interface.ip() {
                if !ip.is_loopback() && !ip.is_unspecified() && !ip.is_link_local() {
                    urls.push(format!("http://{ip}:{port}/"));
                }
            }
        }
    }
    urls[1..].sort();
    urls.dedup();
    urls
}

fn authenticated(authorization: Option<&str>, password_hash: Option<&[u8]>) -> bool {
    let Some(expected) = password_hash else {
        return true;
    };
    let Some((scheme, token)) = authorization.and_then(|s| s.split_once(' ')) else {
        return false;
    };
    if !scheme.eq_ignore_ascii_case("Basic") {
        return false;
    }
    let Ok(decoded) = STANDARD.decode(token.trim()) else {
        return false;
    };
    let Some(colon) = decoded.iter().position(|b| *b == b':') else {
        return false;
    };
    let actual = Sha256::digest(&decoded[colon + 1..]);
    actual
        .iter()
        .zip(expected)
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn allowed_name(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('.')
        && name != "node_modules"
        && !name.contains(['\\', ':', '\0'])
}

/// Decode exactly once; reject traversal on both Unix and Windows before touching disk.
fn resolve_file(root: &Path, encoded: &str) -> Option<PathBuf> {
    let rel = percent_encoding::percent_decode_str(encoded)
        .decode_utf8()
        .ok()?;
    let path = Path::new(rel.as_ref());
    if path.is_absolute() || rel.split('/').any(|part| !allowed_name(part)) {
        return None;
    }
    let mut full = root.to_owned();
    for component in path.components() {
        let Component::Normal(part) = component else {
            return None;
        };
        full.push(part);
        if fs::symlink_metadata(&full).ok()?.file_type().is_symlink() {
            return None;
        }
    }
    let full = full.canonicalize().ok()?;
    (full.starts_with(root) && full.is_file() && file_kind(&full) != "canvas").then_some(full)
}

#[derive(Serialize)]
struct Catalog {
    name: String,
    entries: Vec<Entry>,
    stats: Stats,
}
#[derive(Serialize)]
struct Entry {
    path: String,
    kind: &'static str,
    size: u64,
    modified: Option<u64>,
}
#[derive(Default, Serialize)]
struct Stats {
    files: u64,
    notes: u64,
    folders: u64,
    bytes: u64,
}

fn file_kind(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "md" | "markdown" => "markdown",
        "canvas" => "canvas",
        "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "avif" => "image",
        _ => "file",
    }
}

fn catalog(root: &Path, name: &str) -> Result<Catalog, ()> {
    let mut result = Catalog {
        name: name.into(),
        entries: Vec::new(),
        stats: Stats::default(),
    };
    for item in walkdir::WalkDir::new(root)
        .min_depth(1)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            entry.depth() == 0
                || (entry.file_name().to_str().is_some_and(allowed_name)
                    && !entry.file_type().is_symlink())
        })
    {
        let item = item.map_err(|_| ())?;
        let canonical = item.path().canonicalize().map_err(|_| ())?;
        if !canonical.starts_with(root) {
            continue;
        }
        let metadata = item.metadata().map_err(|_| ())?;
        let kind = if metadata.is_dir() {
            "folder"
        } else if metadata.is_file() {
            file_kind(item.path())
        } else {
            continue;
        };
        if kind == "canvas" {
            continue;
        }
        let size = if metadata.is_file() {
            metadata.len()
        } else {
            0
        };
        if kind == "folder" {
            result.stats.folders += 1;
        } else {
            result.stats.files += 1;
            result.stats.bytes += size;
            if kind == "markdown" {
                result.stats.notes += 1;
            }
        }
        result.entries.push(Entry {
            path: item
                .path()
                .strip_prefix(root)
                .map_err(|_| ())?
                .to_str()
                .ok_or(())?
                .replace('\\', "/"),
            kind,
            size,
            modified: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|duration| duration.as_secs()),
        });
    }
    result.entries.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(result)
}

fn header(name: &str, value: &str) -> Header {
    Header::from_bytes(name, value).expect("static HTTP header")
}

fn respond(request: Request, status: u16, mime: &str, body: Vec<u8>, original: bool) {
    send_response(request, status, mime, Response::from_data(body), original);
}

fn send_response<R: std::io::Read>(
    request: Request,
    status: u16,
    mime: &str,
    response: Response<R>,
    original: bool,
) {
    let mut response = response
        .with_status_code(StatusCode(status))
        .with_header(header("Content-Type", mime))
        .with_header(header("X-Content-Type-Options", "nosniff"))
        .with_header(header("Cache-Control", "no-store"))
        .with_header(header("Referrer-Policy", "no-referrer"))
        .with_header(header("X-Frame-Options", "DENY"))
        .with_header(header(
            "Content-Security-Policy",
            if original {
                "sandbox; default-src 'none'; frame-ancestors 'none'"
            } else {
                "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob: https: http:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'"
            },
        ));
    let explicit_download = request
        .url()
        .split_once('?')
        .is_some_and(|(_, query)| query.split('&').any(|part| part == "download=1"));
    if original && (!mime.starts_with("image/") || explicit_download) {
        response.add_header(header("Content-Disposition", "attachment"));
    }
    if status == 405 {
        response.add_header(header("Allow", "GET"));
    }
    let _ = request.respond(response);
}

fn handle_request(
    mut request: Request,
    root: &Path,
    name: &str,
    password_hash: Option<&[u8]>,
    stop: Arc<AtomicBool>,
) {
    if stop.load(Ordering::Acquire) {
        return;
    }
    let route = request.url().split('?').next().unwrap_or("").to_owned();
    if route == "/share.css" && request.method() == &Method::Get {
        respond(
            request,
            200,
            "text/css; charset=utf-8",
            include_bytes!("../../src/share/share.css").to_vec(),
            false,
        );
        return;
    }
    if route == "/login.js" && request.method() == &Method::Get {
        respond(
            request,
            200,
            "text/javascript; charset=utf-8",
            include_bytes!("../../src/share/login.js").to_vec(),
            false,
        );
        return;
    }
    if route == "/login" && request.method() == &Method::Post {
        use std::io::Read;
        let mut body = String::new();
        let valid_length = request.body_length().is_some_and(|n| n <= 4096);
        let read_ok = valid_length
            && request
                .as_reader()
                .take(4097)
                .read_to_string(&mut body)
                .is_ok();
        let password = body
            .split('&')
            .find_map(|part| part.strip_prefix("password="))
            .unwrap_or("")
            .replace('+', " ");
        let decoded = percent_encoding::percent_decode_str(&password)
            .decode_utf8()
            .ok();
        let valid = read_ok
            && decoded.is_some_and(|p| {
                password_hash.is_none_or(|h| Sha256::digest(p.as_bytes()).as_slice() == &h[..32])
            });
        if valid {
            let token = password_hash
                .map(|h| STANDARD.encode(&h[32..]))
                .unwrap_or_default();
            let response = Response::empty(StatusCode(303))
                .with_header(header("Location", "/"))
                .with_header(header("Cache-Control", "no-store"))
                .with_header(header(
                    "Set-Cookie",
                    &format!("easymd_session={token}; Path=/; HttpOnly; SameSite=Strict"),
                ));
            let _ = request.respond(response);
        } else {
            respond(
                request,
                401,
                "text/html; charset=utf-8",
                include_str!("../../src/share/login.html")
                    .replace("{{error}}", "密码错误，请重试")
                    .into_bytes(),
                false,
            );
        }
        return;
    }
    let session_ok = password_hash.is_some_and(|h| {
        h.len() > 32
            && request
                .headers()
                .iter()
                .filter(|header| header.field.equiv("Cookie"))
                .any(|header| {
                    header.value.as_str().split(';').any(|part| {
                        part.trim()
                            .strip_prefix("easymd_session=")
                            .is_some_and(|token| token == STANDARD.encode(&h[32..]))
                    })
                })
    });
    let authorization = request
        .headers()
        .iter()
        .find(|h| h.field.equiv("Authorization"))
        .map(|h| h.value.as_str());
    if !session_ok && !authenticated(authorization, password_hash) {
        let is_page = route == "/" || route == "/index.html";
        respond(
            request,
            if is_page { 200 } else { 401 },
            "text/html; charset=utf-8",
            include_str!("../../src/share/login.html")
                .replace("{{error}}", "")
                .into_bytes(),
            false,
        );
        return;
    }
    if request.method() != &Method::Get {
        respond(
            request,
            405,
            "text/plain; charset=utf-8",
            b"Read only".to_vec(),
            false,
        );
        return;
    }
    let (mime, body) = match route.as_str() {
        "/" | "/index.html" => (
            "text/html; charset=utf-8",
            include_str!("../../src/share/index.html")
                .as_bytes()
                .to_vec(),
        ),
        "/share.js" => (
            "text/javascript; charset=utf-8",
            include_str!("../../src/share/share.js").as_bytes().to_vec(),
        ),
        "/share.css" => (
            "text/css; charset=utf-8",
            include_str!("../../src/share/share.css")
                .as_bytes()
                .to_vec(),
        ),
        "/api/catalog" => {
            match catalog(root, name).and_then(|c| serde_json::to_vec(&c).map_err(|_| ())) {
                Ok(body) => ("application/json; charset=utf-8", body),
                Err(_) => {
                    respond(
                        request,
                        500,
                        "text/plain; charset=utf-8",
                        "无法读取知识库目录".as_bytes().to_vec(),
                        false,
                    );
                    return;
                }
            }
        }
        _ => {
            if let Some(path) = route
                .strip_prefix("/files/")
                .and_then(|p| resolve_file(root, p))
            {
                if let Ok(file) = fs::File::open(&path) {
                    let ext = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_ascii_lowercase();
                    let mime = if matches!(ext.as_str(), "md" | "markdown" | "txt" | "canvas") {
                        "text/plain; charset=utf-8"
                    } else {
                        crate::mime_of(&ext)
                    };
                    let length = file
                        .metadata()
                        .ok()
                        .and_then(|m| usize::try_from(m.len()).ok());
                    let response = Response::new(
                        StatusCode(200),
                        Vec::new(),
                        CancelReader { reader: file, stop },
                        length,
                        None,
                    );
                    send_response(request, 200, mime, response, true);
                    return;
                }
            }
            respond(
                request,
                404,
                "text/plain; charset=utf-8",
                b"Not found".to_vec(),
                false,
            );
            return;
        }
    };
    if !stop.load(Ordering::Acquire) {
        respond(request, 200, mime, body, false);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpStream;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("easymd-share-{}", uuid::Uuid::new_v4()));
            fs::create_dir_all(root.join("中文目录")).unwrap();
            fs::write(root.join("中文目录/note.md"), "# Note").unwrap();
            fs::write(root.join("board.canvas"), "{}").unwrap();
            fs::write(root.join("image.png"), [0u8, 1, 2]).unwrap();
            fs::write(root.join("unsafe.html"), "<script>alert(1)</script>").unwrap();
            fs::create_dir(root.join(".private")).unwrap();
            fs::write(root.join(".private/secret.md"), "secret").unwrap();
            fs::create_dir(root.join("node_modules")).unwrap();
            fs::write(root.join("node_modules/secret.md"), "secret").unwrap();
            Self(root.canonicalize().unwrap())
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            // Only this test's UUID-named temporary directory is removed.
            assert!(self
                .0
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("easymd-share-"));
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn free_port() -> u16 {
        TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
            .unwrap()
            .local_addr()
            .unwrap()
            .port()
    }
    fn request(port: u16, method: &str, path: &str, auth: Option<&str>) -> String {
        let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n"
        )
        .unwrap();
        if let Some(auth) = auth {
            write!(stream, "Authorization: {auth}\r\n").unwrap();
        }
        write!(stream, "\r\n").unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        response
    }

    #[test]
    fn auth_accepts_any_username_and_entire_password_only() {
        let digest = Sha256::digest("密:码".as_bytes());
        let good = format!("Basic {}", STANDARD.encode("anyone:密:码"));
        assert!(authenticated(Some(&good), Some(&digest)));
        assert!(authenticated(None, None));
        for bad in [
            None,
            Some("Basic !!!"),
            Some("Bearer password"),
            Some("Basic dXNlcjpiYWQ="),
        ] {
            assert!(!authenticated(bad, Some(&digest)));
        }
    }

    #[test]
    fn paths_reject_traversal_hidden_absolute_and_symlinks() {
        let f = Fixture::new();
        assert!(resolve_file(&f.0, "%E4%B8%AD%E6%96%87%E7%9B%AE%E5%BD%95/note.md").is_some());
        for path in [
            "../secret",
            "%2e%2e/secret",
            "/etc/passwd",
            "%2Fetc/passwd",
            "C:%5csecret",
            "a%5c..%5csecret",
            ".private/secret.md",
            "node_modules/secret.md",
            "%00",
            "%FF",
            "中文目录/./note.md",
        ] {
            assert!(resolve_file(&f.0, path).is_none(), "accepted {path}");
        }
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(std::env::temp_dir(), f.0.join("escape")).unwrap();
            std::os::unix::fs::symlink(f.0.join("board.canvas"), f.0.join("alias.canvas")).unwrap();
            assert!(resolve_file(&f.0, "alias.canvas").is_none());
            assert!(resolve_file(&f.0, "escape/outside.txt").is_none());
            assert!(!catalog(&f.0, "test")
                .unwrap()
                .entries
                .iter()
                .any(|e| e.path.starts_with("escape") || e.path == "alias.canvas"));
        }
    }

    #[test]
    fn static_archive_contains_site_catalog_and_only_safe_vault_files() {
        let f = Fixture::new();
        let output =
            std::env::temp_dir().join(format!("easymd-export-{}.zip", uuid::Uuid::new_v4()));
        write_static_archive(&f.0, &output, "Example").unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(&output).unwrap()).unwrap();
        let names = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_owned())
            .collect::<Vec<_>>();
        assert!(names.contains(&"index.html".to_owned()));
        assert!(names.contains(&"share.js".to_owned()));
        assert!(names.contains(&"share.css".to_owned()));
        assert!(names.contains(&"catalog.json".to_owned()));
        assert!(names.contains(&"files/中文目录/note.md".to_owned()));
        assert!(names.contains(&"files/image.png".to_owned()));
        assert!(!names
            .iter()
            .any(|name| name.contains(".private") || name.contains("node_modules")));
        assert!(names.contains(&"files/unsafe.html".to_owned()));
        let mut html = String::new();
        zip.by_name("index.html")
            .unwrap()
            .read_to_string(&mut html)
            .unwrap();
        assert!(html.contains("data-static=\"true\""));
        assert!(!html.contains("/export.zip"));
        assert!(!include_str!("../../src/share/index.html").contains("data-static=\"true\""));
        let mut catalog_json = String::new();
        zip.by_name("catalog.json")
            .unwrap()
            .read_to_string(&mut catalog_json)
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&catalog_json).unwrap();
        assert_eq!(value["stats"]["files"], 3);
        assert!(!names.iter().any(|name| name.ends_with(".canvas")));
        fs::remove_file(&output).unwrap();
    }

    #[test]
    fn catalog_counts_full_visible_vault() {
        let f = Fixture::new();
        let c = catalog(&f.0, "Example").unwrap();
        assert_eq!(c.name, "Example");
        assert_eq!(c.stats.files, 3);
        assert_eq!(c.stats.notes, 1);
        assert_eq!(c.stats.folders, 1);
        assert_eq!(c.stats.bytes, 6 + 3 + 25);
        assert_eq!(c.entries.len(), 4);
        assert!(!c.entries.iter().any(|e| e.kind == "canvas"));
        fs::write(f.0.join("UPPER.CANVAS"), "{}").unwrap();
        assert!(resolve_file(&f.0, "UPPER.CANVAS").is_none());
        assert_eq!(catalog(&f.0, "Example").unwrap().stats.files, 3);
        assert!(c.entries.iter().any(|e| e.kind == "image"));
    }

    #[test]
    fn password_form_session_and_restart() {
        use std::io::{Read, Write};
        fn exchange(port: u16, payload: &str) -> String {
            let mut socket = std::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            socket.write_all(payload.as_bytes()).unwrap();
            let mut output = String::new();
            socket.read_to_string(&mut output).unwrap();
            output
        }
        let f = Fixture::new();
        let service = ShareState::default();
        let port = free_port();
        service
            .start(f.0.clone(), port, "a+b &密码".into(), Ipv4Addr::LOCALHOST)
            .unwrap();
        let page = request(port, "GET", "/", None);
        assert!(page.contains("name=\"password\""));
        assert!(!page.contains("name=\"username\""));
        assert!(!page.to_lowercase().contains("www-authenticate"));
        assert!(page.contains("<script src=\"/login.js\"></script>"));
        let login_script = request(port, "GET", "/login.js", None);
        assert!(login_script.starts_with("HTTP/1.1 200"));
        assert!(login_script.contains("text/javascript; charset=utf-8"));
        assert!(login_script.contains("script-src 'self'"));
        assert!(login_script.contains("share-language"));
        assert!(!login_script.contains("name=\"password\""));
        assert!(request(port, "POST", "/login.js", None).starts_with("HTTP/1.1 401"));
        assert!(request(port, "GET", "/share.js", None).starts_with("HTTP/1.1 401"));
        assert!(request(port, "GET", "/api/catalog", None).starts_with("HTTP/1.1 401"));
        let bad = "password=wrong";
        let failed = exchange(port, &format!("POST /login HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{}", bad.len(), bad));
        assert!(failed.starts_with("HTTP/1.1 401"));
        assert!(!failed.to_lowercase().contains("set-cookie:"));
        let body = "password=a%2Bb+%26%E5%AF%86%E7%A0%81";
        let login = exchange(port, &format!("POST /login HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{}", body.len(), body));
        assert!(login.starts_with("HTTP/1.1 303"), "{login}");
        let cookie = login
            .lines()
            .find(|l| l.to_lowercase().starts_with("set-cookie:"))
            .unwrap()
            .split_once(':')
            .unwrap()
            .1
            .trim()
            .split(';')
            .next()
            .unwrap();
        let get = format!("GET /api/catalog HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\nCookie: {cookie}\r\n\r\n");
        assert!(exchange(port, &get).starts_with("HTTP/1.1 200"));
        service.stop().unwrap();
        service
            .start(f.0.clone(), port, "a+b &密码".into(), Ipv4Addr::LOCALHOST)
            .unwrap();
        assert!(exchange(port, &get).starts_with("HTTP/1.1 401"));
        service.stop().unwrap();
    }

    #[test]
    fn http_auth_all_routes_read_only_and_stop_releases_port() {
        let f = Fixture::new();
        let service = ShareState::default();
        let port = free_port();
        let status = service
            .start(f.0.clone(), port, "password".into(), Ipv4Addr::LOCALHOST)
            .unwrap();
        assert!(status.running);
        let serialized = serde_json::to_string(&status).unwrap();
        assert!(serialized.contains("vaultName"));
        assert!(!serialized.contains("password"));
        let auth = format!("Basic {}", STANDARD.encode(":password"));
        for path in [
            "/share.js",
            "/api/catalog",
            "/files/board.canvas",
            "/missing",
        ] {
            let denied = request(port, "GET", path, None);
            assert!(denied.starts_with("HTTP/1.1 401"), "{denied}");
            assert!(!denied.to_lowercase().contains("www-authenticate"));
        }
        assert!(
            request(port, "GET", "/api/catalog", Some("Basic dXNlcjp3cm9uZw=="))
                .starts_with("HTTP/1.1 401")
        );
        assert!(request(port, "GET", "/", Some(&auth)).starts_with("HTTP/1.1 200"));
        let note = request(
            port,
            "GET",
            "/files/%E4%B8%AD%E6%96%87%E7%9B%AE%E5%BD%95/note.md",
            Some(&auth),
        );
        assert!(note.starts_with("HTTP/1.1 200"));
        assert!(note.ends_with("# Note"));
        let catalog_response = request(port, "GET", "/api/catalog", Some(&auth));
        assert!(catalog_response.starts_with("HTTP/1.1 200"));
        let json: serde_json::Value =
            serde_json::from_str(catalog_response.split("\r\n\r\n").nth(1).unwrap()).unwrap();
        assert_eq!(json["stats"]["files"], 3);
        assert!(
            request(port, "GET", "/files/board.canvas", Some(&auth)).starts_with("HTTP/1.1 404")
        );
        assert!(request(port, "POST", "/api/catalog", Some(&auth)).starts_with("HTTP/1.1 405"));
        assert!(
            request(port, "GET", "/files/%2e%2e/secret", Some(&auth)).starts_with("HTTP/1.1 404")
        );
        let html = request(port, "GET", "/files/unsafe.html", Some(&auth));
        assert!(html.starts_with("HTTP/1.1 200"));
        assert!(html
            .to_lowercase()
            .contains("content-disposition: attachment"));
        assert!(html.contains("sandbox; default-src 'none'"));
        assert!(!service.stop().unwrap().running);
        assert_eq!(service.status().unwrap().port, None);
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        assert!(request(port, "GET", "/api/catalog", None).starts_with("HTTP/1.1 200"));
        service.stop().unwrap();
        let rebound = TcpListener::bind((Ipv4Addr::LOCALHOST, port));
        assert!(rebound.is_ok());
    }

    #[test]
    fn shared_images_are_inline_and_explicit_downloads_are_attachments() {
        let f = Fixture::new();
        let service = ShareState::default();
        let port = free_port();
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        let image = request(port, "GET", "/files/image.png", None);
        assert!(image.starts_with("HTTP/1.1 200"));
        assert!(image.to_lowercase().contains("content-type: image/png"));
        assert!(
            !image
                .to_lowercase()
                .contains("content-disposition: attachment"),
            "Images must display inline: {image}"
        );
        assert!(image.contains("sandbox; default-src 'none'"));
        let download = request(port, "GET", "/files/image.png?download=1", None);
        assert!(download
            .to_lowercase()
            .contains("content-disposition: attachment"));
        let html = request(port, "GET", "/files/unsafe.html", None);
        assert!(html
            .to_lowercase()
            .contains("content-disposition: attachment"));
        service.stop().unwrap();
    }

    #[test]
    fn slow_download_does_not_block_stop() {
        let f = Fixture::new();
        let large = fs::File::create(f.0.join("large.bin")).unwrap();
        large.set_len(128 * 1024 * 1024).unwrap();
        let port = free_port();
        let service = ShareState::default();
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(3)))
            .unwrap();
        write!(
            client,
            "GET /files/large.bin HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        let mut first_byte = [0];
        client.read_exact(&mut first_byte).unwrap();
        let started = std::time::Instant::now();
        service.stop().unwrap();
        let elapsed = started.elapsed();
        assert!(elapsed < Duration::from_secs(1), "stop took {elapsed:?}");
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        service.stop().unwrap();
        drop(client);
    }

    #[test]
    fn port_conflicts_do_not_change_state_and_drop_stops() {
        let f = Fixture::new();
        let occupied = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
        let port = occupied.local_addr().unwrap().port();
        let service = ShareState::default();
        assert!(service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .is_err());
        assert!(!service.status().unwrap().running);
        assert!(service
            .start(f.0.clone(), 0, String::new(), Ipv4Addr::LOCALHOST)
            .is_err());
        drop(occupied);
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        assert!(service
            .start(f.0.clone(), port, "new".into(), Ipv4Addr::LOCALHOST)
            .is_err());
        drop(service);
        assert!(TcpListener::bind((Ipv4Addr::LOCALHOST, port)).is_ok());
    }

    #[test]
    fn restart_serves_promptly_with_stalled_downloads() {
        let f = Fixture::new();
        fs::File::create(f.0.join("restart-large.bin"))
            .unwrap()
            .set_len(128 * 1024 * 1024)
            .unwrap();
        let service = ShareState::default();
        let port = free_port();
        let mut stalled = Vec::new();
        for cycle in 0..3 {
            service
                .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
                .unwrap();
            for _ in 0..4 {
                let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
                client
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                write!(client, "GET /files/restart-large.bin HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n").unwrap();
                client.read_exact(&mut [0]).unwrap();
                stalled.push(client);
            }
            // Keep every old client alive across restarts, without consuming its response.
            thread::sleep(Duration::from_millis(100));
            let stopped = std::time::Instant::now();
            service.stop().unwrap();
            eprintln!("restart cycle {cycle}: stop {:?}", stopped.elapsed());
            service
                .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
                .unwrap();
            let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            client
                .set_read_timeout(Some(Duration::from_millis(750)))
                .unwrap();
            let started = std::time::Instant::now();
            write!(
                client,
                "GET /api/catalog HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            let mut response = String::new();
            let result = client.read_to_string(&mut response);
            let elapsed = started.elapsed();
            eprintln!("restart cycle {cycle}: first GET {elapsed:?}, read={result:?}");
            assert!(
                result.is_ok(),
                "restarted GET stalled after {elapsed:?}: {result:?}"
            );
            assert!(response.starts_with("HTTP/1.1 200"), "{response}");
            assert!(elapsed < Duration::from_millis(750));
            service.stop().unwrap();
        }
    }

    #[test]
    fn restart_with_idle_keepalive_and_incomplete_requests() {
        let f = Fixture::new();
        let service = ShareState::default();
        let port = free_port();
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        let mut clients = Vec::new();
        for _ in 0..6 {
            let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            client
                .set_read_timeout(Some(Duration::from_millis(750)))
                .unwrap();
            write!(
                client,
                "GET /api/catalog HTTP/1.1\r\nHost: localhost\r\nConnection: keep-alive\r\n\r\n"
            )
            .unwrap();
            let mut header = Vec::new();
            while !header.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                client.read_exact(&mut byte).unwrap();
                header.push(byte[0]);
            }
            assert!(header.starts_with(b"HTTP/1.1 200"));
            let length: usize = String::from_utf8(header)
                .unwrap()
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length: ")
                        .map(str::to_owned)
                })
                .unwrap()
                .parse()
                .unwrap();
            client.read_exact(&mut vec![0; length]).unwrap();
            clients.push(client);
        }
        let started = std::time::Instant::now();
        assert!(request(port, "GET", "/api/catalog", None).starts_with("HTTP/1.1 200"));
        eprintln!(
            "GET with six idle keepalive clients: {:?}",
            started.elapsed()
        );
        assert!(started.elapsed() < Duration::from_millis(750));
        // Also leave a partial header and partial body alive while stopping.
        for bytes in [
            "GET / HTTP/1.1\r\nHost:",
            "POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100000\r\n\r\nx",
        ] {
            let mut client = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
            client.write_all(bytes.as_bytes()).unwrap();
            clients.push(client);
        }
        thread::sleep(Duration::from_millis(30));
        let stopped = std::time::Instant::now();
        service.stop().unwrap();
        assert!(stopped.elapsed() < Duration::from_secs(1));
        service
            .start(f.0.clone(), port, String::new(), Ipv4Addr::LOCALHOST)
            .unwrap();
        assert!(request(port, "GET", "/api/catalog", None).starts_with("HTTP/1.1 200"));
        service.stop().unwrap();
    }
}

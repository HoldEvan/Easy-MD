<div align="center">

<img src="docs/images/logo.png" width="96" alt="EasyMD logo" />

# EasyMD

**本地优先的 Markdown 知识库桌面应用 / A local-first Markdown knowledge base**

磁盘上的 `.md` 文件就是全部数据 —— 无插件系统 · 无账号体系 · 本地文件存储

[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS-blue)](https://github.com/FortC/Easy-MD)
[![Framework](https://img.shields.io/badge/Tauri-2-orange)](https://tauri.app)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)

[简体中文](#简体中文) · [English](#english)

![editor](docs/images/editor-dark.png)

</div>

---

## 简体中文

EasyMD 是一款桌面端 Markdown 知识库：一个本地文件夹就是一个库（vault），笔记使用 `.md` / `.markdown`，画布使用 `.canvas`，手动图谱使用 `.graph` JSON 文件。没有账号、没有插件系统——**磁盘文件是唯一真相源，索引只是可随时重建的缓存**。

技术栈：Tauri 2 + Vue 3 + TypeScript + CodeMirror 6 + markdown-it + force-graph，后端索引引擎与文件 IO 全部为 Rust 实现。

### 核心特性

- **知识库 Vault**：一个本地文件夹即一个库；多库创建 / 打开 / 切换；文件树右键新建 / 重命名 / 删除（进系统回收站）；图片默认保存到本地附件目录，也可配置 S3 / PicGo。
- **索引引擎（Rust）**：后台扫描解析 frontmatter / 标题 / 标签 / 双链，缓存存于系统配置目录（vault 之外），可随时删除重建；文件监听实时增量更新。
- **三种编辑模式**：源码（CodeMirror 6）/ 阅读 / 左右分屏（分隔条可拖拽调宽、双击复位），`Ctrl+E` 循环切换。
- **双向链接**：`[[笔记]]`、`[[笔记|别名]]`、`[[笔记#标题]]`、`[[笔记#^块id]]`；编辑器内高亮 + 自动补全；点击未解析链接可创建笔记；右侧反链面板。
- **嵌入语法**：`![[笔记]]`（整篇 / 标题 / 块）、`![[图片.png]]`。
- **图谱视图**：自动笔记关系图、手动节点与连线、绑定笔记、快照和 AI 生成；支持节点搜索定位、适配全图、当前笔记 1–3 层局部关系、孤立点与未解析节点开关。索引支持双链和普通 Markdown 内链，排除代码示例与附件，歧义目标不随意连线。详见 [图谱说明](docs/graph.md)。

![graph](docs/images/graph.png)

- **Canvas 画布**：`.canvas` 文件（Obsidian 兼容格式子集）随 vault 存放；文本 / 笔记 / 图片三类卡片；笔记卡片双击编辑**直写源 md 文件**（双向绑定）；卡片拖拽缩放连线。

![canvas](docs/images/canvas.png)

- **标签系统**：正文 `#tag` + frontmatter `tags`，多级 `#a/b/c`，左侧标签树点击筛选。
- **YAML 属性面板**：可视化编辑别名 / 标签 / 自定义字段，直接写回 md 头部。
- **模板与每日笔记**：自定义模板目录，支持 `{{title}}` / `{{date}}` / `{{time}}` 变量；日记快捷键一键创建。
- **搜索**：`Ctrl+P` 文件名 / 别名快速打开，`Ctrl+Shift+F` 全文搜索（跳转到行）。

![search](docs/images/search.png)

- **导出**：md（原样保留 `[[ ]]` 标记）/ HTML（单文件、链接转换、图片内联）/ PDF（打印另存为）。
- **MCP 云同步（可选）**：通过 MCP 协议对接外部云服务（腾讯云 COS、百度网盘等 MCP 服务端），软件只做 MCP 客户端；多套配置切换、手动 / 定时同步、进度与失败提示；**不配置完全不影响离线使用**。
- **主题**：Obsidian 风格暗 / 亮双主题（CSS 变量），支持自定义 CSS 片段。

![light](docs/images/editor-light.png)

- **系统集成**：Windows 资源管理器右键打开；macOS 原生红黄绿窗口按钮与 Finder 文件关联（`.md` / `.markdown` / `.canvas`）。支持按文件所在目录打开，或临时打开；临时打开可读取原文件目录中的本地图片。

### 分享、图片与 AI

- **本地分享**：编辑器「更多 → 分享」，可设置端口（默认 80）与可选密码；提供局域网地址、二维码和浏览器打开按钮。访客只需密码，无需账号，访问为只读。关闭弹窗不会停止服务。
- **静态部署**：分享弹窗底部「导出部署包」生成 ZIP，可解压到 NGINX 静态站点目录。分享与导出排除画布、隐藏文件、符号链接及 `node_modules`。静态包不继承分享密码，公开部署时需在服务器另行配置访问控制。详见 [本地分享说明](docs/local-sharing.md)。
- **阅读增强**：任务列表、Mermaid 图表、代码块语言与复制反馈；分享页支持双链、图片、阅读模式和手机布局。
- **图片预览与上传**：源码编辑器悬停 Markdown 图片链接或 HTML `img` 可预览本地 / 网络图片；粘贴、拖放或快捷栏选择图片使用全局本地 / S3 / PicGo 配置，上传失败可重试或转存本地。S3 密钥存入系统凭据库。详见 [图片上传说明](docs/image-upload.md)。
- **AI 设置**：支持检索服务端模型列表，检索成功后直接弹出选项，也可手动输入模型名。实际可用模型由所配置服务决定。
- **日历**：按日期查看笔记、随手记、画布、图谱、图片统计，并可生成 AI 日志。

### 快捷键

| Windows | macOS | 功能 |
|---|---|---|
| `Ctrl+E` | `⌘+E` | 循环切换编辑模式（源码 → 分屏 → 阅读） |
| `Ctrl+P` | `⌘+P` | 快速打开（文件名 / 别名搜索） |
| `Ctrl+Shift+F` | `⌘+Shift+F` | 全文搜索 |
| `Ctrl+N` | `⌘+N` | 新建笔记（可选模板） |
| `Ctrl+D` | `⌘+D` | 打开 / 创建今日笔记 |
| `Ctrl+G` | `⌘+G` | 图谱视图 |
| `Ctrl+,` | `⌘+,` | 设置 |
| `Ctrl+S` | `⌘+S` | 立即保存（编辑器也会自动保存） |
| `Ctrl+F5` | `⌘+Shift+F5` | 在编辑器中打开分享弹窗 |
| `Ctrl+B` / `Ctrl+I` | `⌘+B` / `⌘+I` | 源码编辑器加粗 / 斜体 |

Mac 的功能键若默认控制系统功能，需同时按 `Fn`（例如 `Fn+⌘+Shift+F5`），或在系统设置中启用标准 F1–F12。分享快捷键只打开弹窗，不会自动启动服务。

### 快速上手

1. Windows 需要 WebView2 运行时；macOS 使用系统 WebView。当前验证过的 Mac 安装包为 Apple Silicon（M 系列）。
2. Windows 使用安装包或绿色版 exe；macOS 打开 DMG，将 EasyMD 拖入 Applications。可查看 [Releases](https://github.com/FortC/Easy-MD/releases) 中实际提供的产物，或按下文自行构建。
3. 首次启动选择「打开知识库文件夹」，指向任意一个存放 md 文件的目录即可。
4. 想先看看效果？本仓库自带一个演示库 [docs/demo-vault](docs/demo-vault)，用 EasyMD 打开这个文件夹就能体验全部功能。

### 开发与打包

建议使用 Node.js 22 LTS（本项目也曾在 Node 20.14 上验证）及 Rust stable。Windows 需要 MSVC 工具链与 C++ Build Tools；macOS 需要 Xcode Command Line Tools（`xcode-select --install`）。先确保 `node`、`npm`、`cargo` 在 PATH 中。

安装锁定依赖：

```bash
npm ci
```

**Windows：**

```bash
npm run tauri dev
npm run tauri build
```

**macOS：**
显式加载平台配置，以启用原生红黄绿按钮、文件关联和 app / dmg 打包。

```bash
npm run tauri dev -- --config src-tauri/tauri.macos.conf.json
npm run tauri build -- --config src-tauri/tauri.macos.conf.json
```

构建默认使用本机架构；Apple Silicon 上生成 arm64 包。Intel / Universal 包需要相应 Rust target 和单独构建验证，目前未作为已验收产物提供。

默认产物（`<version>` 为当前版本，`<arch>` 为目标架构）：

```text
src-tauri/target/release/EasyMD.exe
src-tauri/target/release/bundle/nsis/EasyMD_<version>_<arch>-setup.exe
src-tauri/target/release/bundle/macos/EasyMD.app
src-tauri/target/release/bundle/dmg/EasyMD_<version>_aarch64.dmg
```

如需将缓存和 Rust 产物放在项目内，可在 macOS 项目根目录执行：

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_HOME="$PWD/.cargo-home"
export CARGO_TARGET_DIR="$PWD/.cargo-target"
export npm_config_cache="$PWD/.npm-cache"
npm ci
npm run tauri build -- --config src-tauri/tauri.macos.conf.json
```

此时产物位于 `.cargo-target/release/bundle/`。改变 `CARGO_HOME` 会使用独立依赖缓存，首次构建需重新下载依赖；常规开发无需设置这些变量。

本地构建未配置 Developer ID 签名与 Apple 公证，不等于可直接公开分发的已公证应用。首次打开若被 macOS 拦截，确认来源后按系统的「隐私与安全性 → 仍要打开」流程处理。正式分发应另行配置签名与公证。

**验证：**

```bash
npm run build
npm run test:share
node scripts/test-renderer.mjs
node scripts/test-image-insertion.mjs
node scripts/test-graph.mjs
node scripts/test-graph-links.mjs
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml
```

`npm run dev` / `npm run build` 会先生成分享站点资源；只有前端构建不会生成桌面安装包。桌面打包使用上面的 `tauri build` 命令。

### 架构速览

```
src/                前端（视图层，不直接碰磁盘）
  stores/           Pinia 状态（vault / 索引镜像 / 编辑器 / 设置 / 同步）
  lib/markdown/     markdown-it 管线（wikilink / 嵌入 / 标签 / 导出渲染）
  lib/export.ts     md / html / pdf 导出
  components/       编辑器 / 文件树 / 标签 / 图谱 / 画布 / 搜索 / 设置
src-tauri/src/      Rust 后端
  index/            索引引擎：parser（frontmatter / 链接 / 标签）+ engine（扫描 / 缓存 / 增量）+ watcher（notify）
  commands/         tauri 命令：vault / fs / 搜索 / 索引 / 设置 / MCP
  mcp/              MCP 客户端（JSON-RPC over stdio）+ 双向同步引擎
  lib.rs            命令注册 + emdasset:// 资源协议（只读 vault 内文件）
```

### 数据边界

- vault 内保存 `.md` / `.markdown` 笔记、`.canvas` 画布、`.graph` 手动图谱及附件。配置与索引缓存不放入 vault。
- 设置、vault 列表、索引缓存、CSS 片段：Windows 位于 `%APPDATA%/com.easymd.app/`，macOS 位于 `~/Library/Application Support/com.easymd.app/`。S3 密钥使用系统凭据库，不写入 vault。
- 删除缓存目录后重新打开 vault 会自动全量重建索引，笔记内容不受影响。

### MCP 同步说明

同步前请在 设置 → 云同步 添加 MCP 服务（command / args / env，与常见 MCP 客户端配置一致），点「测试连接」确认工具可用。软件按工具名自动绑定 列表 / 下载 / 上传 / 删除 操作（可在配置中手动指定）。同步策略：双向新增 / 更新（内容不同时较新者胜，云端无 mtime 时本地优先）；**不自动传播删除**以防误删，删除请在两侧手动操作。

### 许可证

[MIT](LICENSE)

---

## English

EasyMD is a desktop knowledge base for Markdown: one local folder is one vault, and notes use `.md` / `.markdown`, canvases use `.canvas`, and manual graphs use `.graph` JSON files. No accounts, no plugin system — **files on disk are the single source of truth; the index is just a cache you can rebuild at any time**.

Built with Tauri 2 + Vue 3 + TypeScript + CodeMirror 6 + markdown-it + force-graph. The indexing engine and all file IO are written in Rust.

### Highlights

- **Vaults**: one local folder per vault; create / open / switch between multiple vaults; new / rename / delete from the file tree (delete goes to the system recycle bin); images use local attachments by default, with optional S3 / PicGo upload.
- **Rust index engine**: parses frontmatter / headings / tags / wikilinks in the background; the cache lives in the system config dir (outside the vault) and can be deleted and rebuilt at any time; a file watcher keeps it incrementally up to date.
- **Three edit modes**: source (CodeMirror 6), reading, and split view with a draggable divider (double-click to reset); cycle with `Ctrl+E`.
- **Bidirectional links**: `[[note]]`, `[[note|alias]]`, `[[note#heading]]`, `[[note#^block-id]]`; highlighted with autocomplete in the editor; click an unresolved link to create the note; backlinks panel on the right.
- **Embeds**: `![[note]]` (whole note / heading / block), `![[image.png]]`.
- **Graph view**: automatic note relationships and editable manual graphs, note binding, snapshots and AI generation; search and locate nodes, fit the graph, explore 1–3 neighborhood levels, and toggle orphans or unresolved nodes. Both wikilinks and Markdown links are indexed; code examples and attachments are excluded. See [graph guide](docs/graph.md).

- **Canvas**: `.canvas` files (an Obsidian-compatible subset of the JSON format) live in the vault; text / note / image cards; double-clicking a note card edits **the source md file directly** (two-way binding); drag, resize, and connect cards.

- **Tags**: inline `#tag` plus frontmatter `tags`, nested `#a/b/c`, filter via the left tag tree.
- **YAML properties panel**: edit aliases / tags / custom fields visually; written straight back to the note header.
- **Templates & daily notes**: custom template folder with `{{title}}` / `{{date}}` / `{{time}}` variables; one shortcut for today's note.
- **Search**: `Ctrl+P` quick open by file name / alias, `Ctrl+Shift+F` full-text search with jump-to-line.

- **Export**: md (keeps `[[ ]]` markers as-is) / HTML (single file, converted links, inlined images) / PDF (via print).
- **Optional MCP sync**: connect external cloud services (Tencent COS, Baidu Netdisk, …) over the MCP protocol — EasyMD is only an MCP client; multiple profiles, manual or scheduled sync, progress and failure reporting; **works fully offline if you never configure it**.
- **Themes**: Obsidian-style dark & light themes via CSS variables, plus custom CSS snippets.

- **System integration**: Windows Explorer integration; native macOS window controls and Finder associations for `.md` / `.markdown` / `.canvas`. Open the containing folder as a vault, or use temporary mode with local images resolved from the source directory.

### Sharing, images and AI

- **Local sharing**: open More → Share in the editor. Configure a port (default 80) and optional password; copy a LAN URL, scan its QR code or open the browser. Visitors have read-only access with password-only login. Closing the dialog does not stop sharing.
- **Static deployment**: Export deployment package creates a ZIP for an NGINX static site. Canvas files, hidden files, symlinks and `node_modules` are excluded. The exported site does not retain the live sharing password; configure access control on your server if needed. See [sharing guide](docs/local-sharing.md).
- **Reading**: task lists, Mermaid diagrams and code-copy feedback; the sharing site supports wikilinks, images, reading mode and narrow screens.
- **Images**: hover over Markdown image links or HTML `img` in the source editor to preview images. Paste, drop or choose an image using a global local / S3 / PicGo configuration; failed uploads can be retried or saved locally. S3 secrets use the OS credential store. See [image upload guide](docs/image-upload.md).
- **AI settings**: retrieve model IDs from your configured service and choose from the popup, or enter a model manually.
- **Calendar**: daily file statistics and AI log generation.

### Keyboard shortcuts

| Windows | macOS | Action |
|---|---|---|
| `Ctrl+E` | `⌘+E` | Cycle source / split / reading mode |
| `Ctrl+P` | `⌘+P` | Quick open |
| `Ctrl+Shift+F` | `⌘+Shift+F` | Full-text search |
| `Ctrl+N` | `⌘+N` | New note |
| `Ctrl+D` | `⌘+D` | Today's note |
| `Ctrl+G` | `⌘+G` | Graph view |
| `Ctrl+,` | `⌘+,` | Settings |
| `Ctrl+S` | `⌘+S` | Save now |
| `Ctrl+F5` | `⌘+Shift+F5` | Open Share from the editor |
| `Ctrl+B` / `Ctrl+I` | `⌘+B` / `⌘+I` | Bold / italic in the source editor |

On Macs using media keys, also hold `Fn`, or enable standard F1–F12 keys in system settings. The Share shortcut opens the dialog; it does not start the service automatically.

### Getting started

1. Windows requires WebView2; macOS uses its system WebView. Mac packages have been verified on Apple Silicon.
2. Use a Windows installer / portable exe, or open the macOS DMG and drag EasyMD into Applications. Check [Releases](https://github.com/FortC/Easy-MD/releases) for available artifacts, or build below.
3. On first launch, choose "Open vault folder" and point it at any folder of markdown files.
4. Want a quick tour? The repo ships a demo vault at [docs/demo-vault](docs/demo-vault) — open that folder with EasyMD and try everything.

### Development and packaging

Use Node.js 22 LTS (also tested with Node 20.14) and Rust stable. Windows needs the MSVC toolchain and C++ Build Tools; macOS needs Xcode Command Line Tools (`xcode-select --install`). Ensure Node, npm and Cargo are on PATH.

```bash
npm ci
# Windows
npm run tauri dev
npm run tauri build
# macOS: load the native window and app/DMG configuration
npm run tauri dev -- --config src-tauri/tauri.macos.conf.json
npm run tauri build -- --config src-tauri/tauri.macos.conf.json
```

Default artifacts:

```text
src-tauri/target/release/EasyMD.exe
src-tauri/target/release/bundle/nsis/EasyMD_<version>_<arch>-setup.exe
src-tauri/target/release/bundle/macos/EasyMD.app
src-tauri/target/release/bundle/dmg/EasyMD_<version>_aarch64.dmg
```

Builds target the host architecture by default. Apple Silicon produces arm64 packages; Intel / Universal builds need the relevant Rust targets and separate validation. Local builds are not configured for Developer ID signing or Apple notarization; configure both for public distribution.

Optional project-local caches and build directory on macOS:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
export CARGO_HOME="$PWD/.cargo-home"
export CARGO_TARGET_DIR="$PWD/.cargo-target"
export npm_config_cache="$PWD/.npm-cache"
npm ci
npm run tauri build -- --config src-tauri/tauri.macos.conf.json
```

With these overrides, bundles are under `.cargo-target/release/bundle/`. An isolated `CARGO_HOME` downloads dependencies again on its first build; these overrides are optional.

Validation:

```bash
npm run build
npm run test:share
node scripts/test-renderer.mjs
node scripts/test-image-insertion.mjs
node scripts/test-graph.mjs
node scripts/test-graph-links.mjs
cargo check --manifest-path src-tauri/Cargo.toml --all-targets
cargo test --manifest-path src-tauri/Cargo.toml
```

Frontend dev/build scripts regenerate the sharing site assets first. Use `tauri build` to create desktop installers; `npm run build` alone only builds frontend assets.

### Architecture

```
src/                Frontend (views only — never touches disk directly)
  stores/           Pinia state (vault / index mirror / editor / settings / sync)
  lib/markdown/     markdown-it pipeline (wikilinks / embeds / tags / export)
  lib/export.ts     md / html / pdf export
  components/       editor / file tree / tags / graph / canvas / search / settings
src-tauri/src/      Rust backend
  index/            index engine: parser (frontmatter / links / tags) + engine (scan / cache / delta) + watcher (notify)
  commands/         tauri commands: vault / fs / search / index / settings / MCP
  mcp/              MCP client (JSON-RPC over stdio) + two-way sync engine
  lib.rs            command registry + emdasset:// asset protocol (read-only, vault files only)
```

### Data boundaries

- Vaults hold `.md` / `.markdown` notes, `.canvas` canvases, `.graph` manual graphs and attachments. Settings and index caches live outside the vault.
- Settings, the vault list, index cache and CSS snippets live in `%APPDATA%/com.easymd.app/` on Windows or `~/Library/Application Support/com.easymd.app/` on macOS. S3 secrets use the OS credential store.
- Delete the cache dir and reopen a vault: the index rebuilds from scratch; your notes are untouched.

### About MCP sync

Add an MCP service under Settings → Cloud Sync (command / args / env, same shape as common MCP client configs) and hit "Test connection" to verify the tools. EasyMD binds list / download / upload / delete operations by tool name (manually overridable). Sync strategy: two-way for additions and updates (newest wins; local wins when the cloud lacks mtime); **deletions are never propagated automatically** — delete on both sides manually on purpose.

### License

[MIT](LICENSE)

// 平台判定：仅在 WebView 里读 UA，不碰文件系统（业务 IO 仍全部走 Rust 命令）。
// macOS 的 WebView 上报 "Macintosh; Intel Mac OS X" 或 "Macintosh; Apple Silicon"，
// 两者都含 "Macintosh"，故用 Macintosh 判定。

const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;

/** 运行在 macOS 上 */
export const isMac = /Macintosh|Mac OS X/i.test(ua);

/** 运行在 Windows 上 */
export const isWindows = /Windows/i.test(ua);

/**
 * 快捷键前缀：macOS 显示 ⌘，Windows/Linux 显示 Ctrl。
 * 仅用于界面上的按键提示文案，不参与实际按键判定（判定已同时接受 ctrlKey/metaKey）。
 */
export const keyMod = isMac ? "⌘" : "Ctrl";

/** 把提示文案里的 Ctrl 换成当前平台的主修饰键（macOS 为 ⌘） */
export function keyHint(s: string): string {
  return isMac ? s.replaceAll("Ctrl", "⌘") : s;
}

/**
 * 平台专有名词替换：同一件事在 Windows / macOS 上叫法不同，
 * 字典只写 Windows 叫法，这里按平台改写，避免为两种叫法各维护一份文案。
 * 仅替换确定同义的词，不做泛化猜测。
 */
const macTerms: ReadonlyArray<readonly [string, string]> = [
  ["回收站", "废纸篓"],
  ["Recycle Bin", "Trash"],
];

export function platformText(s: string): string {
  let out = keyHint(s);
  if (!isMac) return out;
  for (const [from, to] of macTerms) out = out.replaceAll(from, to);
  return out;
}

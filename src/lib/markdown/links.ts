// 链接解析（前端本地索引版，供渲染/补全即时使用；后端 resolveLink 供单独查询）
import type { NoteIndex } from "../../types";

export interface ResolvedTarget {
  path: string | null;
  ambiguous: boolean;
}

const IMAGE_EXT = ["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "ico", "avif"];

export function isImageName(name: string): boolean {
  const lower = name.toLowerCase();
  return IMAGE_EXT.some((e) => lower.endsWith("." + e));
}

/** Resolve a note target without choosing an arbitrary duplicate filename. */
export function resolveTarget(target: string, notes: NoteIndex[], sourcePath?: string): ResolvedTarget {
  const missing: ResolvedTarget = { path: null, ambiguous: false };
  let t = target.trim().replace(/\\/g, "/");
  try { t = decodeURIComponent(t); } catch { /* Preserve literal malformed escapes. */ }
  if (!t || t.includes(":") || t.startsWith("//")) return missing;
  if (/\.(png|jpg|jpeg|gif|webp|svg|bmp|ico|avif|heic|tif|tiff|pdf|canvas|mp3|wav|ogg|m4a|flac|mp4|webm|mov|avi|mkv|zip|rar|7z|tar|gz|doc|docx|xls|xlsx|ppt|pptx)$/i.test(t)) return missing;
  t = t.toLowerCase();
  const directory = (path: string) => path.includes("/") ? path.slice(0, path.lastIndexOf("/")) : "";
  const sourceDir = sourcePath === undefined ? undefined : directory(sourcePath.replace(/\\/g, "/").toLowerCase());
  const normalize = (path: string): string | null => {
    const parts: string[] = [];
    for (const part of path.split("/")) {
      if (!part || part === ".") continue;
      if (part === "..") { if (!parts.length) return null; parts.pop(); }
      else parts.push(part);
    }
    return parts.join("/");
  };
  const choose = (matches: NoteIndex[]): ResolvedTarget => ({
    path: matches.length === 1 ? matches[0].path : null,
    ambiguous: matches.length > 1,
  });
  const exact = (path: string | null) => path === null ? missing : choose(notes.filter(n => {
    const p = n.path.toLowerCase();
    return p === path || p === path + ".md" || p === path + ".markdown";
  }));
  const rooted = t.startsWith("/");
  const relative = t.startsWith("./") || t.startsWith("../");
  if (rooted) return exact(normalize(t));
  if (sourceDir !== undefined) {
    const local = exact(normalize(sourceDir ? `${sourceDir}/${t}` : t));
    if (local.path || local.ambiguous || relative) return local;
  } else if (relative) return missing;
  const root = exact(normalize(t));
  if (root.path || root.ambiguous || t.includes("/")) return root;
  const preferLocal = (matches: NoteIndex[]) => {
    const local = sourceDir === undefined ? [] : matches.filter(n => directory(n.path.toLowerCase()) === sourceDir);
    return choose(local.length ? local : matches);
  };
  const stem = t.replace(/\.(md|markdown)$/i, "");
  const names = notes.filter(n => n.path.split("/").pop()!.replace(/\.(md|markdown)$/i, "").toLowerCase() === stem);
  if (names.length) return preferLocal(names);
  return preferLocal(notes.filter(n => n.aliases.some(a => a.toLowerCase() === t)));
}

/** 图片资源目标 → 相对路径（有路径用路径，无路径拼附件目录） */
export function resolveAssetPath(
  target: string,
  attachmentsDir: string,
  sourcePath: string,
): string {
  const t = target.trim().replace(/\\/g, "/");
  if (!t) return t;
  if (t.includes("/")) return t;
  // 与当前笔记同目录优先？Obsidian 行为：先附件目录
  const dir = attachmentsDir.trim().replace(/^\/+|\/+$/g, "") || "assets";
  return `${dir}/${t}`;
}

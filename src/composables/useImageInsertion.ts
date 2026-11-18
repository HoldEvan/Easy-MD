import { computed, onMounted, onUnmounted, reactive, ref, watch, type Ref } from "vue";
import { isTauri } from "@tauri-apps/api/core";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { open } from "@tauri-apps/plugin-dialog";
import { api } from "../ipc/tauri";
import { getImageUploadConfig, uploadImage } from "../lib/imageUploadApi";
import { useEditorStore } from "../stores/editor";
import { useVaultStore } from "../stores/vault";

const MAX_BYTES = 20 * 1024 * 1024;
const EXTENSIONS = ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico"];
type Target = { root: string; path: string; token: number };
export type ImageInsertionJob = {
  id: number; name: string; target: Target; state: "queued" | "busy" | "failed" | "pending";
  message: string; invalid: boolean; url?: string; local?: boolean;
};
type Source = File | string;
// Retain failures/results when switching away from the editor. Bytes never go to localStorage.
const jobs = reactive<ImageInsertionJob[]>([]);
const sources = new Map<number, Source>();
const buffers = new Map<number, number[]>();
const notice = ref("");
let nextId = 0;
let running = false;
let current: { matches: (target: Target) => boolean; insert: (text: string) => void } | undefined;

function extension(source: Source) {
  const name = typeof source === "string" ? source : source.name;
  const ext = name.split(".").pop()?.toLowerCase() || "";
  if (EXTENSIONS.includes(ext)) return ext;
  if (typeof source !== "string") {
    const mime = source.type.split("/")[1]?.replace("svg+xml", "svg").replace("jpeg", "jpg");
    if (mime && EXTENSIONS.includes(mime)) return mime;
  }
  throw new Error("不支持此图片格式");
}
function alive(job: ImageInsertionJob) { return jobs.includes(job); }
function matches(job: ImageInsertionJob) { return !job.invalid && !!current?.matches(job.target); }
function remove(id: number) {
  const index = jobs.findIndex(job => job.id === id);
  if (index >= 0) jobs.splice(index, 1);
  sources.delete(id);
  buffers.delete(id);
}
function finish(job: ImageInsertionJob) {
  if (!alive(job)) return;
  if (!matches(job)) {
    job.state = "pending";
    job.message = "图片已保存，笔记已切换。回到原笔记后点击重试插入。";
    return;
  }
  // Angle brackets/newlines must not escape the Markdown destination.
  const url = job.url!.replace(/</g, "%3C").replace(/>/g, "%3E").replace(/\r/g, "%0D").replace(/\n/g, "%0A");
  current!.insert(`![](<${url}>)\n`);
  notice.value = `已插入图片：${job.name}`;
  remove(job.id);
}
async function pump() {
  if (running) return;
  running = true;
  try {
    for (;;) {
      const job = jobs.find(item => item.state === "queued");
      if (!job) break;
      job.state = "busy";
      try {
        if (!matches(job)) throw new Error("笔记已切换；请回到原笔记后重试");
        if (job.url) { finish(job); continue; }
        const source = sources.get(job.id)!;
        const ext = extension(source);
        job.message = "正在读取图片…";
        let bytes = buffers.get(job.id);
        if (!bytes) {
          if (typeof source !== "string" && source.size > MAX_BYTES) throw new Error("图片超过 20 MiB，请缩小后重新选择");
          bytes = typeof source === "string" ? await api.readExternalBinary(source) : Array.from(new Uint8Array(await source.arrayBuffer()));
          if (bytes.length > MAX_BYTES) throw new Error("图片超过 20 MiB，请缩小后重新选择");
          if (!bytes.length) throw new Error("图片文件为空");
          if (!alive(job)) continue;
          buffers.set(job.id, bytes);
        }
        // Read global settings for each attempt, including retries.
        const config = job.local ? { provider: "local" } : await getImageUploadConfig();
        if (!alive(job)) continue;
        if (!matches(job)) throw new Error("笔记已切换；图片已保留，请回到原笔记后重试");
        const local = job.local || config.provider === "local";
        job.message = local ? "正在保存到本地附件…" : `正在上传到 ${config.provider === "s3" ? "S3" : "PicGo"}…`;
        job.url = local ? await api.saveImage(bytes, ext, job.target.root) : (await uploadImage(bytes, ext)).url;
        if (!job.url) throw new Error("上传服务未返回图片地址");
        buffers.delete(job.id);
        finish(job);
      } catch (error) {
        if (alive(job)) { job.state = "failed"; job.message = String(error instanceof Error ? error.message : error); }
      }
    }
  } finally { running = false; }
}

export function useImageInsertion(body: Ref<HTMLElement | undefined>, insert: (text: string) => void) {
  const editor = useEditorStore();
  const vault = useVaultStore();
  let mounted = false;
  let unlisten: (() => void) | undefined;
  const target = (): Target => ({ root: vault.root, path: editor.activePath, token: editor.openToken });
  const context = {
    matches: (t: Target) => mounted && vault.opened && t.root === vault.root && t.path === editor.activePath && t.token === editor.openToken && !!t.path,
    insert,
  };
  watch(() => [vault.root, vault.opened, editor.activePath, editor.openToken], () => {
    for (const job of jobs) if (!context.matches(job.target)) job.invalid = true;
  }, { flush: "sync" });
  function enqueue(list: Source[], captured = target()) {
    if (!captured.path || !captured.root) { notice.value = "请先打开一篇笔记再插入图片"; return; }
    for (const source of list) {
      if (jobs.length >= 10) { notice.value = "最多保留 10 张待处理图片，请先处理队列"; break; }
      const id = ++nextId;
      sources.set(id, source);
      jobs.push({ id, name: typeof source === "string" ? source.split(/[\\/]/).pop()! : source.name || "粘贴的图片", target: captured, state: "queued", message: "等待处理…", invalid: !context.matches(captured) });
    }
    void pump();
  }
  function paste(event: ClipboardEvent) {
    const files = Array.from(event.clipboardData?.items || []).filter(item => item.type.startsWith("image/")).map(item => item.getAsFile()).filter((file): file is File => !!file);
    if (!files.length) return;
    event.preventDefault(); event.stopPropagation(); enqueue(files);
  }
  function drop(event: DragEvent) {
    const files = Array.from(event.dataTransfer?.files || []);
    if (!files.length) return;
    event.preventDefault(); event.stopPropagation(); enqueue(files);
  }
  function dragOver(event: DragEvent) { if (event.dataTransfer?.types.includes("Files")) event.preventDefault(); }
  async function choose() {
    const captured = target();
    if (!captured.path) { notice.value = "请先打开一篇笔记再插入图片"; return; }
    try {
      const paths = await open({ multiple: true, directory: false, filters: [{ name: "图片（最大 20 MiB）", extensions: EXTENSIONS }] });
      if (paths) enqueue(Array.isArray(paths) ? paths : [paths], captured);
    } catch (error) { notice.value = `选择图片失败：${String(error)}`; }
  }
  function retry(id: number, local = false) {
    const job = jobs.find(item => item.id === id);
    if (!job || job.state === "busy" || job.state === "queued") return;
    if (job.target.root !== vault.root || job.target.path !== editor.activePath) {
      job.message = `请先打开原知识库中的「${job.target.path}」，再重试`; return;
    }
    job.target = target(); job.invalid = false;
    if (local) { job.local = true; job.url = undefined; }
    job.state = "queued"; void pump();
  }
  onMounted(() => {
    mounted = true; current = context;
    if (isTauri()) {
      void getCurrentWebview().onDragDropEvent(event => {
        if (current !== context || event.payload.type !== "drop" || !body.value) return;
        const position = event.payload.position.toLogical(window.devicePixelRatio);
        const rect = body.value.getBoundingClientRect();
        if (position.x < rect.left || position.x > rect.right || position.y < rect.top || position.y > rect.bottom) return;
        enqueue(event.payload.paths);
      }).then(stop => { if (mounted) unlisten = stop; else stop(); }).catch(error => { notice.value = `拖放暂不可用，请使用“插入图片”：${String(error)}`; });
    }
  });
  onUnmounted(() => {
    mounted = false; unlisten?.();
    if (current === context) current = undefined;
    for (const job of jobs) job.invalid = true;
  });
  return { jobs, notice, busy: computed(() => jobs.some(job => job.state === "busy" || job.state === "queued")), paste, drop, dragOver, choose, retry, cancel: remove };
}

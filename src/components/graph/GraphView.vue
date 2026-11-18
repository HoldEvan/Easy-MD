<template>
  <div class="graph-view">
    <!-- 左侧图谱列表（仿笔记文件树） -->
    <div class="gv-sidebar">
      <div class="emd-panel-header">
        <span>{{ t("rb.graph") }}</span>
        <button class="gv-new-btn" :title="t('gf.newGraphTitle')" @click="openDialog('new')">
          <Icon name="plus" :size="13" />
        </button>
      </div>
      <div class="gv-graph-list">
        <div
          class="emd-tree-item gv-graph-item"
          :class="{ 'is-active': !isManual }"
          :title="t('gf.autoName')"
          @click="onSelectGraph('')"
        >
          <Icon name="share-2" :size="13" class="gv-graph-icon" />
          <span class="gv-graph-name">{{ t("gf.autoName") }}</span>
          <span class="gv-graph-n">{{ autoCount }}</span>
        </div>
        <div
          v-for="g in graphs"
          :key="g.path"
          class="emd-tree-item gv-graph-item"
          :class="{ 'is-active': ui.graphPath === g.path }"
          @click="onSelectGraph(g.path)"
          @contextmenu.prevent="showGraphMenu($event, g.path)"
        >
          <Icon name="file-text" :size="13" class="gv-graph-icon" />
          <span class="gv-graph-name">{{ graphNameOf(g.path) }}</span>
        </div>
        <div v-if="graphs.length === 0" class="gv-list-empty">{{ t("gf.listEmpty") }}</div>
      </div>
    </div>

    <!-- 主区 -->
    <div class="gv-main">
      <div class="gv-toolbar">
        <span class="gv-cur-name" :title="currentName">{{ currentName }}</span>
        <div class="gv-sep" />
        <button class="emd-btn" :title="t('gf.newGraphTitle')" @click="openDialog('new')">
          <Icon name="plus" :size="12" /> {{ t("gf.newGraph") }}
        </button>
        <button class="emd-btn" :title="t('gf.aiTitle')" @click="aiOpen = true">
          <Icon name="play" :size="12" /> {{ t("gf.aiGen") }}
        </button>

        <!-- 手动图谱：编辑模式 + 工具 -->
        <template v-if="isManual">
          <div class="gv-sep" />
          <button class="emd-btn" :class="{ 'emd-btn-accent': editMode }" :title="t('gf.editHint')" @click="toggleEdit">
            <Icon name="pencil" :size="12" /> {{ editMode ? t("gf.editOn") : t("gf.edit") }}
          </button>
          <template v-if="editMode">
            <div class="gv-tools">
              <button
                v-for="tl in tools"
                :key="tl.key"
                class="gv-tool"
                :class="{ 'is-active': tool === tl.key }"
                :title="tl.title"
                @click="setTool(tl.key)"
              >
                {{ tl.label }}
              </button>
            </div>
          </template>
        </template>

        <!-- 自动图谱：原有过滤 + 快照 -->
        <template v-else>
          <div class="gv-sep" />
          <label class="gv-check">
            <input v-model="localOnly" type="checkbox" :disabled="!editor.activePath" /> 当前笔记
          </label>
          <select v-if="localOnly" v-model.number="localDepth" aria-label="关系层数">
            <option v-for="depth in 3" :key="depth" :value="depth">{{ depth }} 层</option>
          </select>
          <label class="gv-check">
            <input v-model="showOrphans" type="checkbox" /> {{ t("gf.orphans") }}
          </label>
          <label class="gv-check">
            <input v-model="showUnresolved" type="checkbox" /> {{ t("gf.unresolved") }}
          </label>
          <button class="emd-btn" :title="t('gf.snapshot')" @click="snapshotAuto">
            <Icon name="download" :size="12" /> {{ t("gf.snapshot") }}
          </button>
        </template>

        <form class="gv-search" @submit.prevent="locateNode">
          <input v-model="searchQuery" aria-label="搜索节点" placeholder="搜索节点" @input="searchIndex = 0" />
          <button class="emd-btn" :disabled="!searchMatches.length" type="submit">定位<span v-if="searchQuery">（{{ searchMatches.length }}）</span></button>
        </form>
        <button class="emd-btn" title="适配全部节点" @click="fitGraph">全图适配</button>
        <span class="gv-count">{{ tf("gf.counts", { n: nodesCount, m: linksCount }) }}</span>
        <button class="emd-btn" :title="t('gf.relayout')" @click="reheat">
          <Icon name="refresh-cw" :size="12" />
        </button>
        <button class="emd-btn" @click="closeGraph">{{ t("gf.back") }}</button>
      </div>

      <div v-if="graphError" class="gv-error" role="alert">{{ graphError }}</div>
      <div class="gv-canvas-wrap">
        <div ref="container" class="gv-canvas" />
        <div v-if="isManual && doc && doc.nodes.length === 0" class="gv-empty">
          {{ t("gf.empty") }}
        </div>
        <div v-if="linkFrom" class="gv-hint">{{ t("gf.linkHint") }}</div>
      </div>
    </div>

    <!-- 图谱右键菜单 -->
    <DropdownMenu :open="graphMenu.open" :x="graphMenu.x" :y="graphMenu.y" :items="graphMenuItems" @select="onGraphMenu" @close="graphMenu.open = false" />

    <!-- 节点右键菜单 -->
    <DropdownMenu :open="nodeMenu.open" :x="nodeMenu.x" :y="nodeMenu.y" :items="nodeMenuItems" @select="onNodeMenu" @close="nodeMenu.open = false" />

    <!-- 输入弹窗：新建/重命名/节点重命名/绑定笔记 -->
    <InputDialog
      v-if="dialog.mode !== 'none'"
      :title="dialogTitle"
      :placeholder="dialogPh"
      :initial="dialog.initial"
      @confirm="onDialogConfirm"
      @cancel="dialog.mode = 'none'"
    />

    <!-- AI 生成弹窗 -->
    <transition name="emd-fade">
      <div v-if="aiOpen" class="emd-modal-bg" @click.self="aiOpen = false">
        <div class="emd-modal gv-ai-modal">
          <div class="gv-ai-title">{{ t("gf.aiTitle") }}</div>
          <textarea
            v-model="aiInput"
            class="gv-ai-input"
            rows="6"
            :placeholder="t('gf.aiPh')"
            @keydown.esc="aiOpen = false"
          />
          <div v-if="aiErr" class="gv-ai-err">{{ aiErr }}</div>
          <div class="gv-ai-actions">
            <button class="emd-btn" @click="aiOpen = false">{{ t("c.cancel") }}</button>
            <button class="emd-btn emd-btn-accent" :disabled="aiRunning" @click="runAi">
              <Icon :name="aiRunning ? 'refresh-cw' : 'play'" :size="12" />
              {{ aiRunning ? t("ai.thinking") : t("gf.aiRun") }}
            </button>
          </div>
        </div>
      </div>
    </transition>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, reactive, ref, watch } from "vue";
import ForceGraphFactory from "force-graph";
import { useNotesIndexStore } from "../../stores/notesIndex";
import { useEditorStore } from "../../stores/editor";
import { useUiStore } from "../../stores/ui";
import { useSettingsStore } from "../../stores/settings";
import { buildAutoGraph, localGraph } from "../../lib/graph/data";
import { resolveTarget } from "../../lib/markdown/links";
import { t, tf } from "../../i18n";
import Icon from "../common/Icon.vue";
import InputDialog from "../common/InputDialog.vue";
import DropdownMenu, { type DropItem } from "../common/DropdownMenu.vue";
import { api } from "../../ipc/tauri";
import type { GraphDoc, GraphEdge, GraphNode } from "../../types";

/* force-graph 类型泛型较严格，这里用宽松接口包装（数据结构仍强类型） */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
type LooseFG = any;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const ForceGraph = ForceGraphFactory as any;

const GRAPH_DIR = "graphs";

const indexStore = useNotesIndexStore();
const editor = useEditorStore();
const ui = useUiStore();
const settings = useSettingsStore();

// ---------- 状态 ----------
const showOrphans = ref(true);
const localOnly = ref(false);
const localDepth = ref(1);
const searchQuery = ref("");
const searchIndex = ref(0);
let resizeObserver: ResizeObserver | undefined;
let unsubscribeSettings: (() => void) | undefined;
let disposed = false;
let loadRevision = 0;
let docPath = "";
let dirty = false;
let saveQueue: Promise<void> = Promise.resolve();
let pendingWrites = 0;
const graphError = ref("");
const showUnresolved = ref(true);
const container = ref<HTMLElement>();
let fg: LooseFG = null;
let renderedData: unknown = null;

/** .graph 文件列表 */
const graphs = ref<{ path: string; mtime: number }[]>([]);
/** 当前手动图谱文档（graphPath 为空 = 自动图谱，doc 为 null） */
const doc = ref<GraphDoc | null>(null);
const editMode = ref(false);
const tool = ref<"select" | "addNode" | "link">("select");
const linkFrom = ref<string | null>(null);
const selectedId = ref<string | null>(null);
/** 添加节点工具暂存的画布坐标 */
let pendingAddPos: { x: number; y: number } | null = null;
let saveTimer: ReturnType<typeof setTimeout> | null = null;
let nodeSeq = 0;

const graphMenu = reactive({ open: false, x: 0, y: 0, target: "" });
const nodeMenu = reactive({ open: false, x: 0, y: 0, target: null as GraphNode | null });
const dialog = reactive({
  mode: "none" as "none" | "new" | "newNodeLabel" | "rename" | "renameNode" | "bind",
  initial: "",
});

const aiOpen = ref(false);
const aiInput = ref("");
const aiRunning = ref(false);
const aiErr = ref("");

const isManual = computed(() => ui.graphPath !== "");
const currentName = computed(() =>
  isManual.value ? ui.graphPath.split("/").pop()!.replace(/\.graph$/i, "") : t("gf.autoName"),
);

const tools = computed(() => [
  { key: "select" as const, label: t("gf.toolSelect"), title: t("gf.toolSelectHint") },
  { key: "addNode" as const, label: t("gf.toolAdd"), title: t("gf.toolAdd") },
  { key: "link" as const, label: t("gf.toolLink"), title: t("gf.toolLink") },
]);

function graphNameOf(path: string): string {
  return path.split("/").pop()!.replace(/\.graph$/i, "");
}

const autoCount = computed(() => indexStore.notes.length);

const graphMenuItems = computed<DropItem[]>(() => [
  { key: "open", label: t("gf.openGraph"), icon: "share-2" },
  { key: "rename", label: t("gf.rename"), icon: "pencil" },
  { key: "sep", label: "", separator: true },
  { key: "delete", label: t("gf.delete"), icon: "trash-2", danger: true },
]);

function showGraphMenu(ev: MouseEvent, path: string) {
  graphMenu.x = ev.clientX;
  graphMenu.y = ev.clientY;
  graphMenu.target = path;
  graphMenu.open = true;
}

async function onGraphMenu(key: string) {
  graphMenu.open = false;
  const path = graphMenu.target;
  if (!path) return;
  if (key === "open") {
    onSelectGraph(path);
  } else if (key === "rename") {
    onSelectGraph(path);
    openDialog("rename", graphNameOf(path));
  } else if (key === "delete") {
    if (!confirm(tf("gf.deleteConfirm", { name: graphNameOf(path) }))) return;
    try {
      await flushSave();
      await api.deletePath(path);
      if (ui.graphPath === path) {
        ui.graphPath = "";
        doc.value = null;
        editMode.value = false;
      }
      await refreshGraphs();
    } catch (e) {
      alert(String(e));
    }
  }
}

// ---------- 数据：自动（笔记引用） ----------
interface GNode {
  id: string;
  title: string;
  degree: number;
  unresolved: boolean;
  x?: number;
  y?: number;
}
interface GLink {
  source: string | GNode;
  target: string | GNode;
}

/** 自动图谱：笔记节点 + 已解析链接边；可选未解析虚拟节点 */
const autoData = computed<{ nodes: GNode[]; links: GLink[] }>(() => {
  const all = buildAutoGraph(indexStore.notes, showUnresolved.value);
  const data = localOnly.value ? localGraph(all, editor.activePath, localDepth.value) : all;
  return { ...data, nodes: data.nodes.filter(n => showOrphans.value || n.degree > 0 || (localOnly.value && n.id === editor.activePath)) };
});

/** 手动图谱：doc → 渲染节点（保留已存坐标，其余交给力导向布局） */
const manualData = computed<{ nodes: (GraphNode & { degree: number })[]; links: GLink[] }>(() => {
  const d = doc.value;
  if (!d) return { nodes: [], links: [] };
  const nodes = d.nodes.map((n) => ({ ...n, degree: 0 }));
  const idSet = new Set(d.nodes.map((n) => n.id));
  const links: GLink[] = d.edges
    .filter((e) => idSet.has(e.source) && idSet.has(e.target))
    .map((e) => ({ source: e.source, target: e.target }));
  for (const l of links) {
    const s = nodes.find((n) => n.id === l.source);
    const tg = nodes.find((n) => n.id === l.target);
    if (s) s.degree++;
    if (tg) tg.degree++;
  }
  return { nodes, links };
});

const currentData = computed(() => (isManual.value ? manualData.value : autoData.value));
const nodesCount = computed(() => currentData.value.nodes.length);
const linksCount = computed(() => currentData.value.links.length);

// ---------- 渲染 ----------
function cssVar(name: string): string {
  return getComputedStyle(document.documentElement).getPropertyValue(name).trim();
}

function render() {
  if (!container.value || disposed) return;
  const accent = cssVar("--graph-node") || cssVar("--text-accent");
  const lineColor = cssVar("--graph-line") || cssVar("--background-modifier-border");
  const textColor = cssVar("--graph-text") || cssVar("--text-muted");
  const bg = cssVar("--background-primary");
  const faintColor = cssVar("--text-faint");
  const mutedColor = cssVar("--text-muted");

  if (!fg) {
    fg = new ForceGraph(container.value);
  }
  fg
      .backgroundColor(bg)
      .nodeLabel((n: GNode & { label?: string }) =>
        isManual.value ? n.label || "" : `${n.title}${n.unresolved ? t("gf.uncreated") : ""}`,
      )
      .nodeVal((n: GNode) => 2 + (n.degree / Math.max(1, maxDegree())) * 8)
      .nodeCanvasObject(
        (n: GNode & { label?: string; note?: string }, ctx: CanvasRenderingContext2D, globalScale: number) => {
          const r = 3 + (n.degree / Math.max(1, maxDegree())) * 6;
          const isSel = n.id === selectedId.value || n.id === linkFrom.value;
          ctx.beginPath();
          ctx.arc(n.x || 0, n.y || 0, r, 0, 2 * Math.PI);
          if (isManual.value) {
            ctx.fillStyle = n.note ? accent : hexA(accent, 0.5);
          } else {
            ctx.fillStyle = n.unresolved ? faintColor : accent;
          }
          ctx.fill();
          if (isSel) {
            ctx.strokeStyle = accent;
            ctx.lineWidth = 2 / globalScale;
            ctx.stroke();
          }
          const label = (isManual.value ? n.label : n.title) ?? "";
          if (isManual.value || globalScale > 1.2 || n.degree > 0) {
            const text = label.length > 14 ? label.slice(0, 13) + "…" : label;
            ctx.font = `${10 / globalScale}px sans-serif`;
            ctx.fillStyle = isManual.value
              ? n.note
                ? textColor
                : hexA(textColor, 0.65)
              : n.unresolved
                ? mutedColor
                : textColor;
            ctx.textAlign = "center";
            ctx.fillText(text, n.x || 0, (n.y || 0) + r + 10 / globalScale);
          }
        },
      )
      .linkColor(() => lineColor)
      .linkWidth(0.8)
      .cooldownTime(2400)
      .onNodeClick((n: GNode & { note?: string }) => {
        if (!isManual.value) {
          if (n.unresolved) {
            if (confirm(t("gf.createConfirm").replace("{name}", n.title))) {
              import("../../stores/vault").then(({ useVaultStore }) =>
                useVaultStore().newNote(null, n.title),
              );
            }
            return;
          }
          openNote(n.id);
          return;
        }
        // 手动图谱
        if (!doc.value) return;
        if (tool.value === "link" && editMode.value) {
          if (!linkFrom.value) {
            linkFrom.value = n.id;
          } else if (linkFrom.value === n.id) {
            linkFrom.value = null;
          } else {
            addEdge(linkFrom.value, n.id);
            linkFrom.value = null;
          }
          render();
          return;
        }
        if (tool.value === "select") {
          if (n.note && !editMode.value) {
            openNote(n.note);
          } else {
            selectedId.value = selectedId.value === n.id ? null : n.id;
            render();
          }
        }
      })
      .onNodeRightClick((n: GNode & { label?: string }, ev: MouseEvent) => {
        if (!isManual.value || !doc.value) return;
        nodeMenu.x = ev.clientX;
        nodeMenu.y = ev.clientY;
        nodeMenu.target = findNode(n.id) || { ...n } as GraphNode;
        nodeMenu.open = true;
      })
      .onLinkClick((edge: { source: GNode; target: GNode }) => {
        if (!isManual.value || !editMode.value) return;
        if (!confirm(t("gf.deleteEdgeConfirm"))) return;
        const s = typeof edge.source === "object" ? edge.source.id : edge.source;
        const tg = typeof edge.target === "object" ? edge.target.id : edge.target;
        doc.value!.edges = doc.value!.edges.filter(
          (e) => !(e.source === s && e.target === tg) && !(e.source === tg && e.target === s),
        );
        mutateSaved();
      })
      .onBackgroundClick((ev: MouseEvent) => {
        if (!isManual.value || !editMode.value || tool.value !== "addNode") return;
        const rect = container.value!.getBoundingClientRect();
        const p = fg.screen2GraphCoords(ev.clientX - rect.left, ev.clientY - rect.top);
        pendingAddPos = { x: p.x, y: p.y };
        openDialog("newNodeLabel");
      })
      .onNodeDragEnd((n: GNode) => {
        if (!isManual.value || !editMode.value) return;
        const target = findNode(n.id);
        if (target) {
          target.x = n.x;
          target.y = n.y;
        }
        scheduleSave();
      });
  fg.enableNodeDrag(isManual.value && editMode.value);
  onResize();
  if (renderedData !== currentData.value) {
    renderedData = currentData.value;
    fg.graphData(JSON.parse(JSON.stringify(currentData.value)));
  }
}

const maximumDegree = computed(() => currentData.value.nodes.reduce((max, n) => Math.max(max, n.degree), 1));
function maxDegree(): number { return maximumDegree.value; }

/** #rrggbb → rgba */
function hexA(hex: string, alpha: number): string {
  const m = hex.replace("#", "");
  if (m.length !== 6) return hex;
  const r = parseInt(m.slice(0, 2), 16);
  const g = parseInt(m.slice(2, 4), 16);
  const b = parseInt(m.slice(4, 6), 16);
  return `rgba(${r},${g},${b},${alpha})`;
}

const searchMatches = computed(() => {
  const query = searchQuery.value.trim().toLocaleLowerCase();
  return query ? currentData.value.nodes.filter(n => {
    const label = "label" in n ? n.label : n.title;
    return `${label} ${n.id}`.toLocaleLowerCase().includes(query);
  }) : [];
});
function locateNode() {
  const matches = searchMatches.value;
  if (!fg || !matches.length) return;
  const match = matches[searchIndex.value++ % matches.length];
  const node = fg.graphData().nodes.find((n: GNode) => n.id === match.id);
  if (!node || !Number.isFinite(node.x) || !Number.isFinite(node.y)) return;
  selectedId.value = node.id;
  fg.centerAt(node.x, node.y, 300);
  fg.zoom(2, 300);
}
function fitGraph() { fg?.zoomToFit(300, 40); }

function reheat() {
  fg?.d3ReheatSimulation();
}

async function openNote(path: string) {
  await flushSave();
  try { await editor.openNote(path); ui.view = "editor"; }
  catch (e) { graphError.value = `无法打开笔记：${e}`; }
}

function closeGraph() {
  void flushSave();
  ui.view = "editor";
}

function onResize() {
  if (fg && container.value) {
    fg.width(Math.max(1, container.value.clientWidth));
    fg.height(Math.max(1, container.value.clientHeight));
  }
}

// ---------- 图谱文件管理 ----------
async function refreshGraphs() {
  try {
    const all = await api.listAllFilesWithMtime();
    graphs.value = all.filter((f) => f.kind === "graph");
  } catch {
    graphs.value = [];
  }
}

function onSelectGraph(key: string) {
  flushSave();
  ui.graphPath = key;
}

async function loadDoc(silent = false) {
  const revision = ++loadRevision;
  const path = ui.graphPath;
  doc.value = null;
  docPath = "";
  dirty = false;
  if (!path) return;
  try {
    await saveQueue;
    const text = await api.readTextFile(path);
    const parsed = JSON.parse(text) as GraphDoc;
    if (!parsed || !Array.isArray(parsed.nodes) || !Array.isArray(parsed.edges)
      || parsed.nodes.some(n => !n || typeof n.id !== "string" || typeof n.label !== "string")
      || new Set(parsed.nodes.map(n => n.id)).size !== parsed.nodes.length
      || parsed.edges.some(e => !e || typeof e.source !== "string" || typeof e.target !== "string")) {
      throw new Error("图谱数据格式无效");
    }
    if (disposed || revision !== loadRevision || path !== ui.graphPath) return;
    docPath = path;
    doc.value = { version: 1, mode: "manual", nodes: parsed.nodes, edges: parsed.edges };
    graphError.value = "";
  } catch (e) {
    if (disposed || revision !== loadRevision || path !== ui.graphPath) return;
    // Keep a failed document absent: never overwrite corrupt data with an empty graph.
    if (!silent) graphError.value = `${t("gf.loadFail")}: ${e}`;
  }
}

/** 同名加序号 */
async function uniquePath(name: string): Promise<string> {
  let rel = `${GRAPH_DIR}/${name}.graph`;
  for (let i = 2; i < 200; i++) {
    // eslint-disable-next-line no-await-in-loop
    if (!(await api.pathExists(rel))) return rel;
    rel = `${GRAPH_DIR}/${name} ${i}.graph`;
  }
  return rel;
}

async function createGraph(name: string, init: GraphDoc) {
  await flushSave();
  const startingPath = ui.graphPath;
  const rel = await uniquePath(name);
  await api.writeTextFile(rel, JSON.stringify(init, null, 2));
  await refreshGraphs();
  if (!disposed && ui.graphPath === startingPath) onSelectGraph(rel);
  return rel;
}

/** 把当前自动图谱布局快照成可编辑 .graph */
async function snapshotAuto() {
  if (!fg) return;
  const live = fg.graphData() as { nodes: GNode[]; links: GLink[] };
  const titleOf = new Map<string, string>();
  for (const n of live.nodes) titleOf.set(n.id, n.title);
  const nodes: GraphNode[] = live.nodes
    .filter((n) => !n.unresolved)
    .map((n) => ({ id: n.id, label: n.title, note: n.id, x: n.x, y: n.y }));
  const idSet = new Set(nodes.map((n) => n.id));
  const edges: GraphEdge[] = [];
  for (const l of live.links) {
    const s = typeof l.source === "object" ? l.source.id : l.source;
    const tg = typeof l.target === "object" ? l.target.id : l.target;
    if (idSet.has(s) && idSet.has(tg)) edges.push({ source: s, target: tg });
  }
  const now = new Date();
  const pad = (x: number) => String(x).padStart(2, "0");
  const stamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}`;
  await createGraph(`${t("gf.autoSnapName")}-${stamp}`, { version: 1, mode: "manual", nodes, edges });
}

// ---------- 手动编辑 ----------
function findNode(id: string): GraphNode | undefined {
  return doc.value?.nodes.find((n) => n.id === id);
}

function newNodeId(): string {
  nodeSeq += 1;
  return `n-${Date.now().toString(36)}-${nodeSeq}`;
}

function toggleEdit() {
  if (!doc.value) return;
  editMode.value = !editMode.value;
  if (!editMode.value) {
    tool.value = "select";
    linkFrom.value = null;
  }
  render();
}

function setTool(k: "select" | "addNode" | "link") {
  tool.value = k;
  linkFrom.value = null;
  render();
}

function addNodeAt(label: string) {
  if (!doc.value) return;
  doc.value.nodes.push({
    id: newNodeId(),
    label,
    ...(pendingAddPos ? { x: pendingAddPos.x, y: pendingAddPos.y } : {}),
  });
  pendingAddPos = null;
  mutateSaved();
}

function addEdge(from: string, to: string) {
  if (!doc.value) return;
  const dup = doc.value.edges.some(
    (e) => (e.source === from && e.target === to) || (e.source === to && e.target === from),
  );
  if (dup || from === to) return;
  doc.value.edges.push({ source: from, target: to });
  mutateSaved();
}

const nodeMenuItems = computed<DropItem[]>(() => {
  const n = nodeMenu.target;
  if (!n) return [];
  const items: DropItem[] = [{ key: "renameNode", label: t("gf.renameNode"), icon: "pencil" }];
  if (n.note) {
    items.push({ key: "openNote", label: t("gf.openNote"), icon: "file-text" });
    items.push({ key: "unbind", label: t("gf.unbind"), icon: "x" });
  } else {
    items.push({ key: "bind", label: t("gf.bindNote"), icon: "link" });
  }
  items.push({ key: "sep", label: "", separator: true });
  items.push({ key: "delNode", label: t("gf.deleteNode"), icon: "trash-2", danger: true });
  return items;
});

async function onNodeMenu(key: string) {
  nodeMenu.open = false;
  const n = nodeMenu.target;
  if (!n) return;
  if (key === "renameNode") {
    openDialog("renameNode", n.label);
  } else if (key === "openNote" && n.note) {
    openNote(n.note);
  } else if (key === "unbind") {
    n.note = undefined;
    mutateSaved();
    render();
  } else if (key === "bind") {
    openDialog("bind");
  } else if (key === "delNode") {
    doc.value!.nodes = doc.value!.nodes.filter((x) => x.id !== n.id);
    doc.value!.edges = doc.value!.edges.filter((e) => e.source !== n.id && e.target !== n.id);
    if (selectedId.value === n.id) selectedId.value = null;
    mutateSaved();
  }
}

// ---------- 输入弹窗 ----------
const dialogTitle = computed(() => {
  switch (dialog.mode) {
    case "new": return t("gf.newGraphTitle");
    case "newNodeLabel": return t("gf.newNodeTitle");
    case "rename": return t("gf.rename");
    case "renameNode": return t("gf.renameNode");
    case "bind": return t("gf.bindNote");
    default: return "";
  }
});
const dialogPh = computed(() => {
  switch (dialog.mode) {
    case "new":
    case "rename": return t("gf.namePh");
    case "newNodeLabel":
    case "renameNode": return t("gf.nodePh");
    case "bind": return t("gf.bindPh");
    default: return "";
  }
});

function openDialog(mode: typeof dialog.mode, initial = "") {
  dialog.mode = mode;
  dialog.initial = initial;
}

async function onDialogConfirm(value: string) {
  const mode = dialog.mode;
  dialog.mode = "none";
  const v = value.trim();
  if (!v) return;
  try {
    if (mode === "new") {
      await createGraph(v, { version: 1, mode: "manual", nodes: [], edges: [] });
    } else if (mode === "newNodeLabel") {
      addNodeAt(v);
    } else if (mode === "rename" && isManual.value) {
      const oldPath = ui.graphPath;
      await flushSave();
      const newPath = await api.renamePath(oldPath, v);
      if (ui.graphPath === oldPath) ui.graphPath = newPath;
      await refreshGraphs();
    } else if (mode === "renameNode" && nodeMenu.target) {
      nodeMenu.target.label = v;
      mutateSaved();
      render();
    } else if (mode === "bind" && nodeMenu.target) {
      const res = resolveTarget(v, indexStore.notes);
      if (!res.path || res.ambiguous) {
        alert(t("gf.bindNotFound"));
        return;
      }
      nodeMenu.target.note = res.path;
      mutateSaved();
      render();
    }
  } catch (e) {
    alert(String(e));
  }
}

// ---------- AI 生成 ----------
async function runAi() {
  const desc = aiInput.value.trim();
  if (!desc || aiRunning.value) return;
  aiRunning.value = true;
  aiErr.value = "";
  try {
    const sys = `你是知识图谱生成器。根据用户描述生成图谱，输出严格 JSON，格式：
{"nodes":[{"label":"节点名"}],"edges":[{"source":"节点名","target":"节点名"}]}
要求：source/target 必须是 nodes 中出现过的 label；节点数控制在 30 个以内；不要输出任何解释、注释或 markdown 代码块。`;
    const result = await api.aiChat(desc, sys);
    const text = result.replace(/```(json)?/gi, "").trim();
    const start = text.indexOf("{");
    const end = text.lastIndexOf("}");
    if (start < 0 || end <= start) throw new Error(t("gf.parseFail"));
    const parsed = JSON.parse(text.slice(start, end + 1)) as {
      nodes?: { label?: string }[];
      edges?: { source?: string; target?: string }[];
    };
    if (!Array.isArray(parsed.nodes) || parsed.nodes.length === 0) throw new Error(t("gf.parseFail"));
    const nodes: GraphNode[] = parsed.nodes
      .filter((n) => n.label && String(n.label).trim())
      .map((n, i) => ({ id: newNodeId(), label: String(n.label).trim() }));
    const labelToId = new Map(nodes.map((n) => [n.label, n.id]));
    const edges: GraphEdge[] = [];
    for (const e of parsed.edges || []) {
      const s = labelToId.get(String(e.source ?? ""));
      const tg = labelToId.get(String(e.target ?? ""));
      if (s && tg && s !== tg && !edges.some((x) => (x.source === s && x.target === tg) || (x.source === tg && x.target === s))) {
        edges.push({ source: s, target: tg });
      }
    }
    const now = new Date();
    const pad = (x: number) => String(x).padStart(2, "0");
    const stamp = `${now.getFullYear()}${pad(now.getMonth() + 1)}${pad(now.getDate())}-${pad(now.getHours())}${pad(now.getMinutes())}`;
    aiOpen.value = false;
    aiInput.value = "";
    await createGraph(`AI-${stamp}`, { version: 1, mode: "manual", nodes, edges });
  } catch (e) {
    aiErr.value = String(e);
  } finally {
    aiRunning.value = false;
  }
}

// ---------- 持久化 ----------
/** 结构变化后保存（同时吸附当前布局坐标） */
function mutateSaved() {
  captureLivePositions();
  scheduleSave();
  render();
}

/** 从 force-graph 实例回写各节点当前坐标，保证保存的布局与所见一致 */
function captureLivePositions() {
  if (!fg || !doc.value) return;
  try {
    const live = fg.graphData() as { nodes: { id: string; x?: number; y?: number }[] };
    const pos = new Map(live.nodes.map((n) => [n.id, { x: n.x, y: n.y }]));
    for (const n of doc.value.nodes) {
      const p = pos.get(n.id);
      if (p) {
        n.x = p.x;
        n.y = p.y;
      }
    }
  } catch {
    /* 布局尚未就绪时忽略 */
  }
}

function scheduleSave() {
  if (!docPath || !doc.value) return;
  dirty = true;
  if (saveTimer) clearTimeout(saveTimer);
  saveTimer = setTimeout(() => void flushSave(), 400);
}

function flushSave(): Promise<void> {
  if (saveTimer) { clearTimeout(saveTimer); saveTimer = null; }
  if (!dirty || !docPath || !doc.value) return saveQueue;
  const path = docPath;
  const document = doc.value;
  const content = JSON.stringify(document, null, 2);
  dirty = false;
  pendingWrites++;
  saveQueue = saveQueue.then(async () => {
    try { await api.writeTextFile(path, content); }
    catch (e) {
      graphError.value = `${t("gf.saveFail")}: ${e}`;
      if (doc.value === document && docPath === path) dirty = true;
    } finally { pendingWrites--; }
  });
  return saveQueue;
}

// ---------- 生命周期 ----------
function onEsc(ev: KeyboardEvent) {
  if (ev.key !== "Escape") return;
  if (aiOpen.value) aiOpen.value = false;
  else if (linkFrom.value) {
    linkFrom.value = null;
    render();
  }
}

function onGraphChanged(e: Event) {
  const path = (e as CustomEvent<string>).detail;
  refreshGraphs();
  // 外部修改且本地无未保存改动时静默重载
  if (path === ui.graphPath && !dirty && !pendingWrites) loadDoc(true);
}

onMounted(() => {
  render();
  resizeObserver = new ResizeObserver(onResize);
  if (container.value) resizeObserver.observe(container.value);
  void refreshGraphs();
  void loadDoc();
  window.addEventListener("resize", onResize);
  window.addEventListener("keydown", onEsc);
  window.addEventListener("emd-graph-changed", onGraphChanged);
  unsubscribeSettings = settings.$subscribe(() => render());
});

onBeforeUnmount(() => {
  void flushSave();
  disposed = true;
  loadRevision++;
  resizeObserver?.disconnect();
  unsubscribeSettings?.();
  window.removeEventListener("resize", onResize);
  window.removeEventListener("keydown", onEsc);
  window.removeEventListener("emd-graph-changed", onGraphChanged);
  fg?._destructor?.();
  fg = null;
});

watch(currentData, () => render());
watch(
  () => ui.graphPath,
  () => {
    void flushSave();
    editMode.value = false;
    tool.value = "select";
    linkFrom.value = null;
    selectedId.value = null;
    nodeMenu.open = false;
    nodeMenu.target = null;
    dialog.mode = "none";
    pendingAddPos = null;
    graphError.value = "";
    void loadDoc();
  },
  { flush: "sync" },
);
</script>

<style scoped>
.graph-view {
  height: 100%;
  display: flex;
  background: var(--background-primary);
  position: relative;
  overflow: hidden;
}
/* 左侧图谱列表（仿笔记侧栏） */
.gv-sidebar {
  width: 240px;
  flex-shrink: 0;
  display: flex;
  flex-direction: column;
  background: var(--background-secondary);
  border-right: 1px solid var(--background-modifier-border);
  overflow: hidden;
}
.gv-new-btn {
  padding: 3px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
}
.gv-new-btn:hover {
  background: var(--background-modifier-hover);
  color: var(--text-normal);
}
.gv-graph-list {
  flex: 1;
  overflow-y: auto;
  padding: 2px 6px;
}
.gv-graph-item {
  gap: 7px;
}
.gv-graph-icon {
  color: var(--text-faint);
  flex-shrink: 0;
}
.gv-graph-item.is-active .gv-graph-icon {
  color: var(--text-accent);
}
.gv-graph-name {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.gv-graph-n {
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.gv-list-empty {
  color: var(--text-faint);
  padding: 16px 10px;
  text-align: center;
  font-size: var(--font-ui-smaller);
  line-height: 1.6;
}
/* 主区 */
.gv-main {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  position: relative;
}
.gv-cur-name {
  font-weight: 600;
  color: var(--text-normal);
  max-width: 260px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.gv-toolbar {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 8px 14px;
  border-bottom: 1px solid var(--background-modifier-border);
  flex-shrink: 0;
  flex-wrap: wrap;
}
.gv-sep {
  width: 1px;
  height: 14px;
  background: var(--background-modifier-border);
  flex-shrink: 0;
}
.gv-search { display: flex; align-items: center; gap: 4px; }
.gv-search input { width: 130px; min-width: 0; }
.gv-error { padding: 6px 14px; color: var(--text-error); }
.gv-check {
  display: flex;
  align-items: center;
  gap: 5px;
  color: var(--text-muted);
  user-select: none;
  white-space: nowrap;
}
.gv-tools {
  display: inline-flex;
  gap: 2px;
  background: var(--background-primary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 2px;
}
.gv-tool {
  height: 22px;
  padding: 0 10px;
  border-radius: var(--radius-s);
  color: var(--text-muted);
  font-size: 12px;
  white-space: nowrap;
  transition: background var(--anim-fast), color var(--anim-fast);
}
.gv-tool:hover {
  color: var(--text-normal);
}
.gv-tool.is-active {
  background: var(--background-modifier-active-hover);
  color: var(--text-normal);
}
.gv-danger:hover {
  color: var(--text-error);
}
.gv-count {
  margin-left: auto;
  color: var(--text-faint);
  font-size: var(--font-ui-smaller);
}
.gv-canvas-wrap {
  flex: 1;
  min-height: 0;
  position: relative;
}
.gv-canvas {
  position: absolute;
  inset: 0;
}
.gv-empty {
  position: absolute;
  left: 50%;
  top: 44%;
  transform: translate(-50%, -50%);
  color: var(--text-faint);
  font-size: var(--font-ui-size);
  text-align: center;
  max-width: 420px;
  line-height: 1.7;
  pointer-events: none;
}
.gv-hint {
  position: absolute;
  top: 12px;
  left: 50%;
  transform: translateX(-50%);
  background: var(--background-secondary);
  border: 1px solid var(--background-modifier-border);
  border-radius: var(--radius-m);
  padding: 5px 14px;
  color: var(--text-accent);
  font-size: var(--font-ui-smaller);
  box-shadow: var(--shadow-l1);
  pointer-events: none;
}
.gv-ai-modal {
  width: min(480px, 90vw);
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
}
.gv-ai-title {
  font-weight: 600;
  color: var(--text-normal);
}
.gv-ai-input {
  resize: vertical;
  font-family: var(--font-text);
}
.gv-ai-err {
  color: var(--text-error);
  font-size: var(--font-ui-smaller);
}
.gv-ai-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>

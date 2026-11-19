import MarkdownIt from 'markdown-it';
import { renderMermaidBlocks } from '../lib/markdown/mermaid';
import { taskListPlugin } from '../lib/markdown/taskList';
import { centerContainers } from './center';
import { htmlImages } from './images';
import { keyboardLabels } from './kbd';
import { wikiLinks } from './wikiLinks';
import { codeBlockControls } from './codeBlocks';
import { copyWithFeedback } from './copyFeedback';
import { applyLanguage, getLanguage, localize, setLanguage, t, type MessageKey, type ShareLanguage } from './i18n';
import { decodeURIComponentSafe, encodedPath, resolveVaultPath, safeHref, stripFrontmatter } from './utils';
import './share.css';

type Entry = { path: string; kind: 'folder' | 'markdown' | 'image' | 'canvas' | 'file'; size: number; modified: number | null };
type Catalog = { name: string; entries: Entry[]; stats: { files: number; notes: number; folders: number; bytes: number } };

const $ = <T extends HTMLElement>(selector: string): T => document.querySelector(selector) as T;
let theme = 'dark';
try { theme = localStorage.getItem('share-theme') === 'light' ? 'light' : 'dark'; } catch { /* Storage may be disabled. */ }
const app = { catalog: null as Catalog | null, selected: '', query: '', expanded: new Set<string>(), theme, static: document.body.dataset.static === 'true' || new URLSearchParams(location.search).has('static') || location.protocol === 'file:' };
const md = new MarkdownIt({ html: false, linkify: true, breaks: false, typographer: false });
md.validateLink = (url: string) => safeHref(url) !== null;
md.use(centerContainers);
md.use(htmlImages);
md.use(keyboardLabels);
md.use(wikiLinks);
taskListPlugin(md);
codeBlockControls(md);
md.renderer.rules.image = (tokens, idx, options, env, renderer) => {
  const token = tokens[idx];
  const src = token.attrGet('src') || '';
  const resolved = resourceUrl(src, env.path || '');
  if (!resolved) return '';
  token.attrSet('src', resolved);
  token.attrSet('alt', renderer.renderInlineAsText(token.children || [], options, env));
  token.attrSet('loading', 'lazy');
  return renderer.renderToken(tokens, idx, options);
};
md.renderer.rules.link_open = (tokens, idx, options, env, renderer) => {
  const token = tokens[idx];
  const href = token.attrGet('href') || '';
  const wiki = href.startsWith('vault:') ? href.slice(6) : href;
  if (wiki && !/^(https?:|mailto:|#)/i.test(wiki)) {
    const path = resolveVaultPath(wiki, env.path || '');
    const entry = findEntry(path);
    if (entry) {
      token.attrSet('href', `#${encodeURIComponent(entry.path)}`);
      token.attrSet('class', 'vault-link');
      return renderer.renderToken(tokens, idx, options);
    }
  }
  if (href.startsWith('#')) {
    token.attrSet('href', '#');
    return renderer.renderToken(tokens, idx, options);
  }
  if (!/^(https?:|mailto:)/i.test(href)) return '<span class="unsafe-link">';
  const safe = safeHref(href);
  if (!safe) return '<span class="unsafe-link">';
  token.attrSet('href', safe);
  token.attrSet('rel', 'noopener noreferrer');
  if (/^https?:/i.test(safe)) token.attrSet('target', '_blank');
  return renderer.renderToken(tokens, idx, options);
};

function resourceUrl(src: string, currentPath: string): string | null {
  if (/^(data:|blob:|https?:)/i.test(src)) return src;
  if (/^(javascript:|file:|\/\/)/i.test(src)) return null;
  const clean = src.split(/[?#]/)[0];
  const path = clean.startsWith('/') ? clean.slice(1) : resolveVaultPath(clean, currentPath);
  return path ? fileUrl(path) : null;
}
function findEntry(path: string): Entry | undefined {
  return app.catalog?.entries.find((entry) => entry.path === path) || app.catalog?.entries.find((entry) => entry.path === `${path}.md`);
}
function formatBytes(value: number): string {
  if (value < 1024) return `${value} B`;
  const units = ['KB', 'MB', 'GB', 'TB']; let n = value / 1024; let i = 0;
  while (n >= 1024 && i < units.length - 1) { n /= 1024; i++; }
  return `${n.toFixed(n >= 10 ? 0 : 1)} ${units[i]}`;
}
function el<K extends keyof HTMLElementTagNameMap>(tag: K, className?: string, text?: string): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag); if (className) node.className = className; if (text !== undefined) node.textContent = text; return node;
}
function ui<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, key: MessageKey, params: Record<string, string | number> = {}): HTMLElementTagNameMap[K] {
  const node = el(tag, className); localize(node, key, params); return node;
}
function errorNode<K extends keyof HTMLElementTagNameMap>(tag: K, className: string, key: 'readError' | 'catalogError', error: unknown): HTMLElementTagNameMap[K] {
  const node = ui(tag, className, key, { error: error instanceof Error ? error.message : t('unknownError') });
  if (!(error instanceof Error)) node.dataset.shareUnknownError = 'true';
  return node;
}
function renderStats(): void {
  if (!app.catalog) return;
  const { stats } = app.catalog; const host = $('#stats'); host.replaceChildren();
  const values: Array<[MessageKey, string]> = [['files', String(stats.files)], ['notes', String(stats.notes)], ['folders', String(stats.folders)], ['bytes', formatBytes(stats.bytes)]];
  for (const [label, value] of values) { const item = el('div', 'stat'); item.append(ui('span', 'stat-label', label), el('strong', 'stat-value', value)); host.append(item); }
  const name = $('#vault-name');
  if (app.catalog.name) { name.removeAttribute('data-share-i18n'); name.textContent = app.catalog.name; }
  else localize(name, 'vault');
  $('#file-count').textContent = String(stats.files);
}
function treeData(): Map<string, Entry[]> {
  const map = new Map<string, Entry[]>();
  for (const entry of app.catalog?.entries || []) {
    const parts = entry.path.split('/'); const parent = parts.slice(0, -1).join('/');
    if (!map.has(parent)) map.set(parent, []); map.get(parent)!.push(entry);
  }
  for (const list of map.values()) list.sort((a, b) => (a.kind === 'folder' ? 0 : 1) - (b.kind === 'folder' ? 0 : 1) || a.path.localeCompare(b.path, 'zh-CN'));
  return map;
}
function renderTree(): void {
  if (!app.catalog) return;
  const host = $('#tree'); host.replaceChildren();
  const map = treeData();
  const matching = (entry: Entry): boolean => {
    if (!app.query || entry.path.toLocaleLowerCase().includes(app.query.toLocaleLowerCase())) return true;
    return entry.kind === 'folder' && (map.get(entry.path) || []).some(matching);
  };
  const renderLevel = (parent: string, depth: number) => {
    for (const entry of map.get(parent) || []) {
      if (!matching(entry)) continue;
      const row = el('button', `tree-row${entry.path === app.selected ? ' active' : ''}`); row.type = 'button'; row.style.paddingLeft = `${10 + depth * 16}px`;
      const isFolder = entry.kind === 'folder'; const children = map.has(entry.path);
      row.append(ui('span', `kind-icon ${entry.kind}`, entry.kind), el('span', 'tree-name', entry.path.split('/').pop() || entry.path));
      if (isFolder) {
        const expanded = app.expanded.has(entry.path) || Boolean(app.query);
        row.setAttribute('aria-expanded', String(expanded));
        const arrow = el('span', 'tree-chevron');
        arrow.setAttribute('aria-hidden', 'true');
        arrow.innerHTML = `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="${expanded ? 'm6 9 6 6 6-6' : 'm9 6 6 6-6 6'}"/></svg>`;
        row.append(arrow);
      }
      row.addEventListener('click', () => {
        if (isFolder) { if (app.expanded.has(entry.path)) app.expanded.delete(entry.path); else app.expanded.add(entry.path); renderTree(); }
        else selectEntry(entry);
      });
      host.append(row);
      if (isFolder && children && (app.expanded.has(entry.path) || app.query)) renderLevel(entry.path, depth + 1);
    }
  };
  renderLevel('', 0);
}
function selectEntry(entry: Entry): void {
  if (location.hash === `#${encodeURIComponent(entry.path)}`) { app.selected = entry.path; renderTree(); void renderFile(entry); return; }
  location.hash = encodeURIComponent(entry.path);
}
function selectHash(): void {
  const path = decodeURIComponentSafe(location.hash.slice(1));
  const entry = findEntry(path);
  if (!entry || entry.kind === 'folder') { app.selected = ''; renderTree(); $('#content').replaceChildren(ui('div', 'welcome', 'welcome')); return; }
  app.selected = entry.path;
  let parent = entry.path.split('/').slice(0, -1).join('/');
  while (parent) { app.expanded.add(parent); parent = parent.split('/').slice(0, -1).join('/'); }
  renderTree(); void renderFile(entry);
}
function routeUrl(path: string): string { return `${app.static ? '' : '/'}${path}`; }
function fileUrl(path: string): string { return routeUrl(`files/${encodedPath(path)}`); }
async function copyText(text: string): Promise<boolean> {
  try {
    if (navigator.clipboard && window.isSecureContext) { await navigator.clipboard.writeText(text); return true; }
  } catch { /* fallback below */ }
  const textarea = document.createElement('textarea');
  textarea.value = text; textarea.setAttribute('readonly', ''); textarea.style.position = 'fixed'; textarea.style.opacity = '0';
  document.body.append(textarea); textarea.select();
  let ok = false;
  try { ok = document.execCommand('copy'); } catch { ok = false; }
  textarea.remove();
  return ok;
}
async function renderFile(entry: Entry): Promise<void> {
  const host = $('#content'); host.replaceChildren(); host.scrollTop = 0;
  const head = el('header', 'file-head'); const title = el('h1', 'file-title', entry.path.split('/').pop() || entry.path); const actions = el('div', 'file-actions');
  const download = ui('a', 'download', 'download'); download.href = fileUrl(entry.path); download.download = entry.path.split('/').pop() || 'download';
  const crumbsRow = el('div', 'breadcrumbs-row');
  const crumbs = el('div', 'breadcrumbs', entry.path);
  if (entry.modified != null) {
    const date = new Date(entry.modified * 1000);
    if (!Number.isNaN(date.getTime())) {
      const updated = el('time', 'file-updated', t('updated', { date: date.toLocaleString(getLanguage(), { hour12: false }) }));
      updated.dateTime = date.toISOString();
      updated.dataset.shareDate = 'true';
      crumbsRow.append(crumbs, updated);
    } else crumbsRow.append(crumbs);
  } else crumbsRow.append(crumbs);
  const titleRow = el('div', 'file-title-row');
  actions.append(download); titleRow.append(title, actions); head.append(titleRow, crumbsRow); host.append(head);
  const body = el('article', 'file-body'); host.append(body);
  try {
    if (entry.kind === 'markdown') {
      const response = await fetch(fileUrl(entry.path)); if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const source = await response.text();
      const html = md.render(stripFrontmatter(source), { path: entry.path, notePaths: (app.catalog?.entries || []).filter(item => item.kind === 'markdown').map(item => item.path) });
      body.innerHTML = html;
      await renderMermaidBlocks(body);
      body.querySelectorAll<HTMLButtonElement>('.code-copy').forEach((button) => {
        button.addEventListener('click', async () => {
          const code = button.closest('pre')?.querySelector('code')?.textContent || '';
          await copyWithFeedback(button, () => copyText(code));
        });
      });
      body.querySelectorAll('a').forEach((anchor) => {
        const href = anchor.getAttribute('href') || '';
        if (href.startsWith('#')) anchor.addEventListener('click', (event) => { event.preventDefault(); const path = decodeURIComponentSafe(href.slice(1)); const target = findEntry(path); if (target) selectEntry(target); });
      });
      body.querySelectorAll('img').forEach((img) => img.addEventListener('error', () => img.classList.add('broken-image')));
    } else if (entry.kind === 'image') {
      const image = el('img', 'preview-image'); image.src = fileUrl(entry.path); image.alt = entry.path; body.append(image);
    } else if (entry.kind === 'canvas') {
      const response = await fetch(fileUrl(entry.path)); if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const data = await response.json() as { nodes?: Array<{ id?: string; type?: string; text?: string; file?: string; x?: number; y?: number }> };
      const nodes = data.nodes || [];
      const summary = ui('p', 'canvas-summary', 'nodes', { count: nodes.length }); body.append(summary);
      const list = el('div', 'canvas-nodes');
      for (const [index, node] of nodes.entries()) {
        const card = el('section', 'canvas-node');
        const indexLabel = el('strong', 'canvas-index');
        indexLabel.append(document.createTextNode(`${index + 1} · `), node.type ? document.createTextNode(node.type) : ui('span', '', 'text'));
        card.append(indexLabel);
        if (node.text) card.append(el('p', '', node.text));
        if (node.file) { const targetPath = resolveVaultPath(node.file, entry.path); const target = findEntry(targetPath); if (target) { const link = el('button', 'canvas-link', target.path); link.type = 'button'; link.addEventListener('click', () => selectEntry(target)); card.append(link); } else card.append(el('span', 'canvas-file', node.file)); }
        list.append(card);
      }
      body.append(list);
    } else {
      const note = ui('p', 'file-download-note', 'noPreview'); body.append(note);
    }
  } catch (error) { body.replaceChildren(errorNode('p', 'error', 'readError', error)); }
}
async function init(): Promise<void> {
  document.documentElement.dataset.theme = app.theme;
  applyLanguage();
  $('#language-select').addEventListener('change', event => {
    setLanguage((event.target as HTMLSelectElement).value as ShareLanguage);
    applyLanguage();
  });
  try {
    const catalogUrl = app.static ? 'catalog.json' : '/api/catalog';
    const response = await fetch(catalogUrl); if (!response.ok) throw new Error(`HTTP ${response.status}`);
    app.catalog = await response.json() as Catalog; renderStats(); renderTree();
    if (location.hash) selectHash();
    window.addEventListener('hashchange', selectHash);
  } catch (error) { const host = $('#content'); host.replaceChildren(errorNode('div', 'error-state', 'catalogError', error)); }
  $('#search').addEventListener('input', (event) => { app.query = (event.target as HTMLInputElement).value.trim(); renderTree(); });
  $('#theme-toggle').addEventListener('click', () => { app.theme = app.theme === 'dark' ? 'light' : 'dark'; document.documentElement.dataset.theme = app.theme; try { localStorage.setItem('share-theme', app.theme); } catch { /* Theme still works for this page. */ } });
  const readingMode = $('#reading-mode');
  readingMode.addEventListener('click', () => {
    const enabled = document.body.classList.toggle('reading-mode');
    readingMode.setAttribute('aria-pressed', String(enabled));
    localize(readingMode, enabled ? 'exitReading' : 'reading');
  });
}
void init();

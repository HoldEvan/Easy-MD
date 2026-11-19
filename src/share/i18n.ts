export type ShareLanguage = 'zh-CN' | 'en';

const messages = {
  'zh-CN': {
    title: '知识库分享', vault: '知识库', language: '语言', theme: '切换主题', reading: '阅读模式', exitReading: '退出阅读',
    navigation: '文件导航', filter: '筛选', search: '输入名称', tree: '文件树', files: '文件', notes: '笔记', folders: '目录', bytes: '容量',
    folder: '目录', markdown: '笔记', image: '图片', canvas: '画布', file: '文件', welcome: '从左侧选择文件开始浏览。',
    download: '下载', updated: '更新时间：{date}', nodes: '{count} 个节点', text: '文本',
    noPreview: '此文件类型不支持在线预览。', readError: '读取失败：{error}', catalogError: '无法载入知识库目录：{error}', unknownError: '未知错误',
    noteMissing: '未找到笔记', copyCode: '复制代码', copied: '已复制', copyFailed: '复制失败',
  },
  en: {
    title: 'Shared vault', vault: 'Vault', language: 'Language', theme: 'Toggle theme', reading: 'Reading mode', exitReading: 'Exit reading mode',
    navigation: 'File navigation', filter: 'Filter', search: 'Enter a name', tree: 'File tree', files: 'Files', notes: 'Notes', folders: 'Folders', bytes: 'Size',
    folder: 'Folder', markdown: 'Note', image: 'Image', canvas: 'Canvas', file: 'File', welcome: 'Select a file from the sidebar to start browsing.',
    download: 'Download', updated: 'Updated: {date}', nodes: '{count} nodes', text: 'Text',
    noPreview: 'This file type cannot be previewed online.', readError: 'Failed to read: {error}', catalogError: 'Unable to load the vault catalog: {error}', unknownError: 'Unknown error',
    noteMissing: 'Note not found', copyCode: 'Copy code', copied: 'Copied', copyFailed: 'Copy failed',
  },
} satisfies Record<ShareLanguage, Record<string, string>>;
export type MessageKey = keyof typeof messages.en;
type Params = Record<string, string | number>;
const storageKey = 'share-language';

/** An explicit preference wins; otherwise follow the first supported browser language. */
export function detectLanguage(saved: string | null, languages: readonly string[]): ShareLanguage {
  if (saved === 'zh-CN' || saved === 'en') return saved;
  for (const language of languages) {
    if (/^zh(?:-|$)/i.test(language)) return 'zh-CN';
    if (/^en(?:-|$)/i.test(language)) return 'en';
  }
  return 'en';
}
let saved: string | null = null;
try { saved = localStorage.getItem(storageKey); } catch { /* Storage may be disabled. */ }
let language = detectLanguage(saved, typeof navigator === 'undefined' ? ['zh-CN'] : navigator.languages?.length ? navigator.languages : [navigator.language]);
export function getLanguage(): ShareLanguage { return language; }
export function setLanguage(value: ShareLanguage): void {
  language = value;
  try { localStorage.setItem(storageKey, value); } catch { /* Language still works for this page. */ }
}
export function t(key: MessageKey, params: Params = {}): string {
  return messages[language][key].replace(/\{(\w+)\}/g, (placeholder, name: string) => String(params[name] ?? placeholder));
}

/** Only explicitly marked interface nodes are translated; note text is never traversed. */
export function localize(node: HTMLElement, key: MessageKey, params: Params = {}, attribute?: 'title' | 'aria-label'): void {
  const suffix = attribute ? `-${attribute}` : '';
  node.setAttribute(`data-share-i18n${suffix}`, key);
  if (!attribute) node.dataset.shareParams = JSON.stringify(params);
  if (attribute) node.setAttribute(attribute, t(key, params));
  else node.textContent = t(key, params);
}
export function applyLanguage(root: ParentNode = document): void {
  document.documentElement.lang = language;
  document.title = t('title');
  for (const suffix of ['', '-title', '-aria-label', '-placeholder']) {
    root.querySelectorAll<HTMLElement>(`[data-share-i18n${suffix}]`).forEach(node => {
      const key = node.getAttribute(`data-share-i18n${suffix}`) as MessageKey;
      const params = node.dataset.shareParams ? JSON.parse(node.dataset.shareParams) as Params : {};
      if (node.dataset.shareUnknownError) params.error = t('unknownError');
      if (suffix) node.setAttribute(suffix.slice(1), t(key, params));
      else node.textContent = t(key, params);
    });
  }
  root.querySelectorAll<HTMLTimeElement>('time[data-share-date]').forEach(node => {
    node.textContent = t('updated', { date: new Date(node.dateTime).toLocaleString(language, { hour12: false }) });
  });
  const picker = document.querySelector<HTMLSelectElement>('#language-select');
  if (picker) picker.value = language;
}

import type MarkdownIt from 'markdown-it';
import { t } from './i18n';

/** Resolve against the public catalog only, including filename-only wiki links. */
export function resolveWikiNote(target: string, paths: string[]): string | null {
  const normalized = target.trim().replace(/\\/g, '/').toLowerCase();
  if (!normalized) return null;
  const exact = paths.find(path => path.toLowerCase() === normalized || path.toLowerCase() === `${normalized}.md`);
  if (exact) return exact;
  const stem = normalized.split('/').pop()!.replace(/\.md$/i, '');
  return paths.find(path => path.split('/').pop()!.replace(/\.md$/i, '').toLowerCase() === stem) ?? null;
}

export function wikiLinks(md: MarkdownIt): void {
  md.inline.ruler.before('link', 'share_wikilink', (state, silent) => {
    const linkLevel = (state as typeof state & { linkLevel?: number }).linkLevel ?? 0;
    if (linkLevel > 0 || state.src.slice(state.pos, state.pos + 2) !== '[[') return false;
    const end = state.src.indexOf(']]', state.pos + 2);
    if (end < 0 || end + 2 > state.posMax) return false;
    const inner = state.src.slice(state.pos + 2, end);
    if (inner.includes('[') || inner.includes('\n')) return false;
    if (!silent) {
      const bar = inner.indexOf('|');
      const target = (bar < 0 ? inner : inner.slice(0, bar)).trim();
      const label = (bar < 0 ? '' : inner.slice(bar + 1).trim()) || target;
      const token = state.push('share_wikilink', '', 0);
      token.meta = { target, label };
    }
    state.pos = end + 2;
    return true;
  });
  md.renderer.rules.share_wikilink = (tokens, idx, _options, env) => {
    const { target, label } = tokens[idx].meta;
    const path = resolveWikiNote(target, env.notePaths || []);
    const text = md.utils.escapeHtml(label);
    return path
      ? `<a class="vault-link" href="#${encodeURIComponent(path)}">${text}</a>`
      : `<span class="wiki-unresolved" data-share-i18n-title="noteMissing" title="${t('noteMissing')}">${text}</span>`;
  };
}

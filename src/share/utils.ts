/** Remove only a closed YAML metadata block at the very start of a document. */
export function stripFrontmatter(source: string): string {
  return source.replace(/^\uFEFF?---[ \t]*\r?\n(?:[^\n]*\n)*?(?:---|\.\.\.)[ \t]*(?:\r?\n|$)/, '');
}

export function safeHref(href: string, base = 'http://localhost/'): string | null {
  try {
    const url = new URL(href, base);
    return ['http:', 'https:', 'mailto:'].includes(url.protocol) ? url.href : null;
  } catch { return null; }
}
export function decodeURIComponentSafe(value: string): string { try { return decodeURIComponent(value); } catch { return value; } }
export function resolveVaultPath(target: string, currentPath = ''): string {
  const noAnchor = target.split('#')[0].split('?')[0].replace(/^\.\//, '');
  const base = noAnchor.startsWith('/') ? [] : currentPath.split('/').slice(0, -1);
  for (const part of noAnchor.split('/')) {
    if (!part || part === '.') continue;
    if (part === '..') base.pop(); else base.push(decodeURIComponentSafe(part));
  }
  return base.join('/');
}
export function encodedPath(path: string): string { return path.split('/').filter(Boolean).map(encodeURIComponent).join('/'); }

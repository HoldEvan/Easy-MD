import type MarkdownIt from 'markdown-it';

/** Recognize only img tags, without enabling arbitrary HTML in shared notes. */
export function htmlImages(md: MarkdownIt): void {
  md.inline.ruler.before('html_inline', 'share_html_image', (state, silent) => {
    const source = state.src.slice(state.pos);
    if (source.startsWith('<!--')) {
      const close = source.indexOf('-->');
      const length = close < 0 ? source.length : close + 3;
      if (!silent) state.push('text', '', 0).content = source.slice(0, length);
      state.pos += length;
      return true;
    }
    const opening = /^<img(?=\s|\/?>)/i.exec(source);
    if (!opening) return false;
    let pos = opening[0].length;
    const attrs = new Map<string, string>();
    while (pos < source.length) {
      const end = /^\s*\/?>/.exec(source.slice(pos));
      if (end) { pos += end[0].length; break; }
      // Consume every attribute, but only copy the explicit safe subset below.
      const attr = /^\s+([^\s"'<>\/=]+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?/.exec(source.slice(pos));
      if (!attr) return false;
      const name = attr[1].toLowerCase();
      if (!attrs.has(name)) attrs.set(name, md.utils.unescapeAll(attr[2] ?? attr[3] ?? attr[4] ?? ''));
      pos += attr[0].length;
    }
    if (source[pos - 1] !== '>') return false;
    const src = md.normalizeLink(attrs.get('src') || '');
    if (!src || !md.validateLink(src)) return false;
    if (!silent) {
      const token = state.push('image', 'img', 0);
      token.attrSet('src', src);
      token.attrSet('alt', attrs.get('alt') || '');
      const alt = new state.Token('text', '', 0);
      alt.content = attrs.get('alt') || '';
      token.children = [alt];
      token.content = alt.content;
      if (attrs.has('title')) token.attrSet('title', attrs.get('title')!);
      for (const name of ['width', 'height']) {
        const value = attrs.get(name);
        if (value && /^\d+(?:\.\d+)?%?$/.test(value)) token.attrSet(name, value);
      }
    }
    state.pos += pos;
    return true;
  });
}

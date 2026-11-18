import type MarkdownIt from 'markdown-it';

/** Allow keyboard labels without enabling arbitrary HTML or attributes. */
export function keyboardLabels(md: MarkdownIt): void {
  md.inline.ruler.before('html_inline', 'share_kbd', (state, silent) => {
    const match = /^<kbd\s*>([^<>]*?)<\/kbd\s*>/i.exec(state.src.slice(state.pos));
    if (!match) return false;
    if (!silent) {
      state.push('kbd_open', 'kbd', 1);
      state.push('text', '', 0).content = md.utils.unescapeAll(match[1]);
      state.push('kbd_close', 'kbd', -1);
    }
    state.pos += match[0].length;
    return true;
  });
}

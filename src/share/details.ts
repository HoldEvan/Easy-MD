import type MarkdownIt from 'markdown-it';

/** Native disclosure widgets, with Markdown content and no arbitrary HTML attributes. */
export function detailsContainers(md: MarkdownIt): void {
  md.block.ruler.before('paragraph', 'details_container', (state, start, end, silent) => {
    const line = (n: number) => state.src.slice(state.bMarks[n] + state.tShift[n], state.eMarks[n]);
    if (state.sCount[start] - state.blkIndent >= 4 || !/^<details(?:\s+open(?:\s*=\s*(?:""|''|"open"|'open'|open))?)?\s*>/i.test(line(start))) return false;
    let depth = 0, fence = '', fenceLength = 0, comment = false;
    let finish = start;
    const lines: string[] = [];
    for (; finish < end; finish++) {
      const text = line(finish);
      lines.push(state.src.slice(state.bMarks[finish] + state.blkIndent, state.eMarks[finish]));
      const marker = /^(?:`{3,}|~{3,})/.exec(text)?.[0];
      if (fence) {
        if (marker?.[0] === fence && marker.length >= fenceLength && text.slice(marker.length).trim() === '') fence = '';
        continue;
      }
      if (marker) { fence = marker[0]; fenceLength = marker.length; continue; }
      if (state.sCount[finish] - state.blkIndent >= 4) continue;
      if (comment) { if (text.includes('-->')) comment = false; continue; }
      if (text.trimStart().startsWith('<!--')) { comment = !text.includes('-->'); continue; }
      // Structural lines only: tag examples inside ordinary text/code do not close containers.
      if (!/^\s*<\/?(?:details|summary)\b/i.test(text)) continue;
      const structural = text.replace(/<summary\s*>[\s\S]*?<\/summary\s*>/gi, '');
      for (const tag of structural.matchAll(/<\/?details\b[^>]*>/gi)) depth += /^<\//.test(tag[0]) ? -1 : 1;
      if (depth === 0) break;
    }
    if (finish === end || depth !== 0) return false;
    const match = /^<details(\s+open(?:\s*=\s*(?:""|''|"open"|'open'|open))?)?\s*>\s*<summary\s*>([\s\S]*?)<\/summary\s*>([\s\S]*)<\/details\s*>\s*$/i.exec(lines.join('\n').trimStart());
    if (!match) return false;
    if (silent) return true;
    const open = state.push('details_open', 'details', 1); open.block = true;
    if (match[1]) open.attrSet('open', '');
    state.push('summary_open', 'summary', 1);
    const title = state.push('inline', '', 0); title.content = match[2].trim(); title.children = [];
    state.push('summary_close', 'summary', -1);
    md.block.parse(match[3].replace(/^\n/, ''), md, state.env, state.tokens);
    const close = state.push('details_close', 'details', -1); close.block = true;
    state.line = finish + 1;
    return true;
  }, { alt: ['paragraph', 'reference', 'blockquote'] });
}

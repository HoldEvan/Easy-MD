import type MarkdownIt from 'markdown-it';

/** Allow only alignment containers; arbitrary HTML remains escaped. */
export function centerContainers(md: MarkdownIt): void {
  const opening = /^<div\s+align\s*=\s*(?:"center"|'center'|center)\s*>\s*$/i;
  const closing = /^<\/div\s*>\s*$/i;
  md.block.ruler.before('paragraph', 'center_container', (state, start, end, silent) => {
    const line = (n: number) => state.src.slice(state.bMarks[n] + state.tShift[n], state.eMarks[n]);
    if (state.sCount[start] - state.blkIndent >= 4 || !opening.test(line(start))) return false;
    let depth = 1, finish = start + 1;
    let fence = '', fenceLength = 0;
    for (; finish < end; finish++) {
      const text = line(finish);
      const marker = /^(?:`{3,}|~{3,})/.exec(text)?.[0];
      if (fence) {
        if (marker?.[0] === fence && marker.length >= fenceLength && text.slice(marker.length).trim() === '') fence = '';
        continue;
      }
      if (marker) { fence = marker[0]; fenceLength = marker.length; continue; }
      if (state.sCount[finish] - state.blkIndent >= 4) continue;
      if (opening.test(text)) depth++;
      if (closing.test(text) && --depth === 0) break;
    }
    if (depth !== 0) return false;
    if (silent) return true;
    const open = state.push('center_open', 'div', 1);
    open.block = true;
    open.attrSet('class', 'md-align-center');
    const oldMax = state.lineMax;
    state.lineMax = finish;
    md.block.tokenize(state, start + 1, finish);
    state.lineMax = oldMax;
    const close = state.push('center_close', 'div', -1);
    close.block = true;
    state.line = finish + 1;
    return true;
  });
}

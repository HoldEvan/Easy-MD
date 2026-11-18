import MarkdownIt from "markdown-it";

const blockParser = new MarkdownIt({ html: true });
/** Preserve literal code and HTML while accepting omitted list-marker spaces. */
export function normalizeTaskMarkers(source: string): string {
  const protectedLines = new Set<number>();
  for (const token of blockParser.parse(source, {})) {
    if (token.map && ["fence", "code_block", "html_block"].includes(token.type)) {
      for (let i = token.map[0]; i < token.map[1]; i++) protectedLines.add(i);
    }
  }
  return source.split("\n").map((line, index) => protectedLines.has(index) ? line : line.replace(/^(\s*(?:>\s*)*)-\[([ xX]?)\](?=\s|$)/, "$1- [$2]")).join("\n");
}

export function taskListPlugin(markdown: MarkdownIt): void {
  markdown.core.ruler.before("block", "compact_tasks", (state) => {
    state.src = normalizeTaskMarkers(state.src);
  });
  markdown.core.ruler.after("inline", "task_checkboxes", (state) => {
    for (let i = 2; i < state.tokens.length; i++) {
      const token = state.tokens[i];
      if (token.type !== "inline" || state.tokens[i - 1].type !== "paragraph_open" || state.tokens[i - 2].type !== "list_item_open") continue;
      const first = token.children?.[0];
      const match = first?.type === "text" ? /^\[([ xX]?)\](?:[ \t]+|$)/.exec(first.content) : null;
      if (!first || !match) continue;
      first.content = first.content.slice(match[0].length);
      const checkbox = new state.Token("html_inline", "", 0);
      checkbox.content = `<input type="checkbox" class="md-task" disabled${match[1].trim() ? " checked" : ""}> `;
      token.children!.unshift(checkbox);
      state.tokens[i - 2].attrJoin("class", "task-list-item");
    }
  });
}

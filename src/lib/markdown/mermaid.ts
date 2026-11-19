import mermaid from "mermaid";
import { t } from "../../i18n";

let sequence = 0;
mermaid.initialize({ startOnLoad: false, securityLevel: "strict", suppressErrorRendering: true });

/** Show diagrams on success; keep source only as a syntax-error fallback. */
export async function renderMermaidBlocks(root: ParentNode): Promise<void> {
  for (const code of root.querySelectorAll<HTMLElement>("pre > code.language-mermaid")) {
    const pre = code.parentElement;
    if (!pre || pre.dataset.mermaidRendered) continue;
    pre.dataset.mermaidRendered = "pending";
    const id = `emd-mermaid-${++sequence}`;
    try {
      const { svg } = await mermaid.render(id, code.textContent || "");
      if (!root.contains(pre)) continue;
      const diagram = document.createElement("div");
      diagram.className = "mermaid-diagram";
      diagram.style.overflowX = "auto";
      diagram.innerHTML = svg;
      for (const attr of ['data-source-line', 'data-source-end']) {
        if (pre.hasAttribute(attr)) diagram.setAttribute(attr, pre.getAttribute(attr)!);
      }
      pre.replaceWith(diagram);
    } catch {
      pre.dataset.mermaidRendered = "error";
      const error = document.createElement("p");
      error.textContent = t("md.mermaidError");
      error.style.color = "var(--text-error)";
      pre.before(error);
      document.getElementById(`d${id}`)?.remove();
    }
  }
}

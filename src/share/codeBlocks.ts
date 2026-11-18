import type MarkdownIt from 'markdown-it';

export function codeBlockControls(md: MarkdownIt): void {
  const fenceOrig = md.renderer.rules.fence!;
  const toolbar = (language: string) => {
    const safeInfo = language.replace(/[&<>\"']/g, (char) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '\"': '&quot;', "'": '&#39;' }[char] || char));
    const label = `<span class="code-language">${safeInfo || 'text'}</span>`;
    return `<div class="code-toolbar">${label}<button type="button" class="code-copy" aria-label="复制代码" title="复制代码"><svg viewBox="0 0 24 24" aria-hidden="true"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V5a1 1 0 0 0-1-1H5a1 1 0 0 0-1 1v10a1 1 0 0 0 1 1h3"/></svg></button></div>`;
  };
  md.renderer.rules.fence = (tokens, idx, options, env, renderer) => {
    const token = tokens[idx];
    const rendered = fenceOrig(tokens, idx, options, env, renderer);
    const info = token.info.trim().split(/\s+/, 1)[0] || '';
    return rendered.replace('<pre>', `<pre>${toolbar(info)}`);
  };
  const codeBlockOrig = md.renderer.rules.code_block!;
  md.renderer.rules.code_block = (tokens, idx, options, env, renderer) => {
    const rendered = codeBlockOrig(tokens, idx, options, env, renderer);
    return rendered.replace('<pre>', `<pre>${toolbar('')}`);
  };
}

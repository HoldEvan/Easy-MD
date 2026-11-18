import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { transform } from 'esbuild';
import MarkdownIt from 'markdown-it';
const source = await readFile(new URL('../src/lib/markdown/taskList.ts', import.meta.url), 'utf8');
const { code } = await transform(source.replace('from "markdown-it"', `from ${JSON.stringify(import.meta.resolve('markdown-it'))}`), { loader: 'ts', format: 'esm' });
const { taskListPlugin } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
const md = new MarkdownIt().use(taskListPlugin);
assert.equal((md.render('- [] pending\n-[x] done').match(/type="checkbox"/g)||[]).length,2);
for (const text of ['- [ ] pending', '- [] empty', '-[x] done', '- [X] done', '> -[x] quote', '- parent\n  - [x] child', '- [ ] **bold**\n\n  paragraph']) {
  assert.match(md.render(text), /type="checkbox"/, text);
}
assert.match(md.render('-[x] done'), /disabled checked/);
assert.equal((md.render('-[x] first\n-[x] second').match(/type="checkbox"/g) || []).length, 2);
for (const text of ['text - [X] prose', '`[x] code`', '```\n-[x] code\n```', '    -[x] code', '[x](https://example.com)', '- `[x]` code']) {
  assert.doesNotMatch(md.render(text), /type="checkbox"/, text);
}
assert.match(md.render('- [] **bold**'), /<strong>bold<\/strong>/);
console.log('Task list standard/compact/nested/code/prose regressions passed');

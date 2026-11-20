import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { build, transform } from 'esbuild';
async function importShareModule(name) {
  const result = await build({ entryPoints: [new URL(`../src/share/${name}.ts`, import.meta.url).pathname], bundle: true, platform: 'node', format: 'esm', write: false });
  return import(`data:text/javascript;base64,${Buffer.from(result.outputFiles[0].contents).toString('base64')}`);
}
const source = await readFile(new URL('../src/share/utils.ts', import.meta.url), 'utf8');
const { code } = await transform(source, { loader: 'ts', format: 'esm' });
const { encodedPath, resolveVaultPath, safeHref, stripFrontmatter } = await import(`data:text/javascript;base64,${Buffer.from(code).toString('base64')}`);
assert.equal(stripFrontmatter('---\ntitle: Test\ntags: [one, two]\n---\n# Body'), '# Body');
assert.equal(stripFrontmatter('\uFEFF---\r\ntitle: Test\r\n...\r\nBody'), 'Body');
for (const text of ['# Body\n---\ntext', '---\ntitle: unclosed', '```yaml\n---\ntitle: code\n---\n```']) assert.equal(stripFrontmatter(text), text);
assert.equal(resolveVaultPath('../assets/p.png', 'notes/doc.md'), 'assets/p.png');
assert.equal(resolveVaultPath('pic.png', 'notes/doc.md'), 'notes/pic.png');
assert.equal(resolveVaultPath('../img/a b.png', 'docs/read.md'), 'img/a b.png');
assert.equal(resolveVaultPath('./note.md', 'docs/read.md'), 'docs/note.md');
assert.equal(encodedPath('目录/a b.md'), '%E7%9B%AE%E5%BD%95/a%20b.md');
assert.equal(safeHref('javascript:alert(1)'), null);
assert.equal(safeHref('data:text/html,x'), null);
assert.equal(safeHref('https://example.com/a'), 'https://example.com/a');
assert.equal(safeHref('/files/a'), 'http://localhost/files/a');
const { default: MarkdownIt } = await import('markdown-it');
const centerSource = await readFile(new URL('../src/share/center.ts', import.meta.url), 'utf8');
const centerCode = await transform(centerSource, { loader: 'ts', format: 'esm' });
const { centerContainers } = await import(`data:text/javascript;base64,${Buffer.from(centerCode.code).toString('base64')}`);
const { codeBlockControls } = await importShareModule('codeBlocks');
const imagesSource = await readFile(new URL('../src/share/images.ts', import.meta.url), 'utf8');
const imagesCode = await transform(imagesSource, { loader: 'ts', format: 'esm' });
const { htmlImages } = await import(`data:text/javascript;base64,${Buffer.from(imagesCode.code).toString('base64')}`);
const kbdSource = await readFile(new URL('../src/share/kbd.ts', import.meta.url), 'utf8');
const kbdCode = await transform(kbdSource, { loader: 'ts', format: 'esm' });
const { keyboardLabels } = await import(`data:text/javascript;base64,${Buffer.from(kbdCode.code).toString('base64')}`);
const { wikiLinks, resolveWikiNote } = await importShareModule('wikiLinks');
const { detailsContainers } = await importShareModule('details');
const md = new MarkdownIt({ html: false }).use(centerContainers).use(detailsContainers).use(codeBlockControls).use(htmlImages).use(keyboardLabels).use(wikiLinks);
assert.match(md.render('<details><summary>16 previous messages</summary>\n\nHidden text\n\n</details>'), /<details>\s*<summary>16 previous messages<\/summary>/);
assert.match(md.render('<details open><summary>**Messages**</summary>body</details>'), /<details open="">\s*<summary><strong>Messages<\/strong><\/summary>/);
assert.match(md.render('<details>\n<summary>Messages</summary>\n\n**Bold**\n\n- Item\n\n</details>'), /<strong>Bold<\/strong>[\s\S]*<li>Item<\/li>/);
const nestedDetails = md.render('<details><summary>Outer</summary>\n<details><summary>Inner</summary>\ncontent\n</details>\n</details>');
assert.equal((nestedDetails.match(/<details>/g) || []).length, 2);
assert.match(md.render('<details><summary>Code</summary>\n```html\n</details>\n```\n</details>'), /&lt;\/details&gt;/);
for (const literal of ['`<details><summary>x</summary></details>`', '```html\n<details><summary>x</summary></details>\n```', '    <details><summary>x</summary></details>', '<!-- <details><summary>x</summary></details> -->', '<details onclick="alert(1)"><summary>x</summary></details>', '<details><summary onclick="alert(1)">x</summary></details>', '<details><summary>x</summary>unclosed']) {
  assert.doesNotMatch(md.render(literal), /<details(?:\s|>)/);
}
assert.doesNotMatch(md.render('<details><summary><script>alert(1)</script></summary>\n<script>alert(1)</script>\n</details>'), /<script>/);
const wikiEnv = { notePaths: ['目录/笔记.md', 'Other.md'] };
assert.equal(resolveWikiNote('笔记', wikiEnv.notePaths), '目录/笔记.md');
assert.equal(resolveWikiNote('目录/笔记.md', wikiEnv.notePaths), '目录/笔记.md');
assert.match(md.render('[[笔记|别名]]', wikiEnv), /href="#%E7%9B%AE%E5%BD%95%2F%E7%AC%94%E8%AE%B0.md">别名<\/a>/);
assert.match(md.render('[[other]]', wikiEnv), /href="#Other.md"/);
assert.match(md.render('[[不存在|缺失]]', wikiEnv), /<span class="wiki-unresolved"[^>]*>缺失<\/span>/);
assert.match(md.render('[[笔记|<script>]]', wikiEnv), /&lt;script&gt;/);
for (const literal of ['`[[笔记|别名]]`', '```\n[[笔记|别名]]\n```', '<!-- [[笔记|别名]] -->']) assert.doesNotMatch(md.render(literal, wikiEnv), /class="vault-link"/);
assert.match(md.render('<kbd>Ctrl</kbd>+<kbd>F5</kbd>'), /<kbd>Ctrl<\/kbd>\+<kbd>F5<\/kbd>/);
assert.match(md.render('<kbd>&lt; &amp; &gt;</kbd>'), /<kbd>&lt; &amp; &gt;<\/kbd>/);
for (const literal of ['`<kbd>Ctrl</kbd>`', '```html\n<kbd>Ctrl</kbd>\n```', '<!-- <kbd>Ctrl</kbd> -->', '<kbd onclick="alert(1)">Ctrl</kbd>']) {
  assert.doesNotMatch(md.render(literal), /<kbd(?:\s|>)/);
}
assert.equal(md.render('<div align="center">\n**居中**\n</div>'), '<div class="md-align-center">\n<p><strong>居中</strong></p>\n</div>\n');
assert.match(md.render('<div align=center>\n![image](pic.png)\n</div>'), /<img src="pic.png"/);
const fenced = md.render('```html\n<div align="center">\ntext\n</div>\n```');
assert.doesNotMatch(fenced, /class="md-align-center"/);
assert.match(fenced, /<pre><div class="code-toolbar"><span class="code-language">html<\/span>/);
assert.match(fenced, /<code class="language-html">&lt;div align=&quot;center&quot;&gt;/);
const inlineLanguage = md.render('```js"><img src=x onerror=alert(1)>\nconst value = 1;\n```');
assert.match(inlineLanguage, /class="language-js&quot;&gt;&lt;img/);
assert.match(inlineLanguage, /<div class="code-toolbar"><span class="code-language">js&quot;&gt;&lt;img<\/span>/);
assert.doesNotMatch(inlineLanguage, /<img src=x/);
assert.match(md.render('```\nplain\n```'), /<button type="button" class="code-copy"/);
const indented = md.render('    const indented = true;\n');
assert.match(indented, /<pre><div class="code-toolbar"><span class="code-language">text<\/span><button type="button" class="code-copy"/);
assert.match(indented, /<code>const indented = true;/);
assert.doesNotMatch(md.render('`<div align="center">`'), /class="md-align-center"/);
assert.doesNotMatch(md.render('<div align="center" onclick="alert(1)">\nx\n</div>'), /class="md-align-center"/);
assert.match(md.render('<div align=center>\n<script>alert(1)</script>\n</div>'), /&lt;script&gt;/);
assert.doesNotMatch(md.render('<div align=center>\nunclosed'), /class="md-align-center"/);
// Regression: html:false used to escape HTML images, including centered ones.
const imageMarkdown = '<div align=center>\n<img src="../assets/p.png" alt="截图" width="320">\n</div>';
assert.match(new MarkdownIt({ html: false }).use(centerContainers).render(imageMarkdown), /&lt;img/);
assert.match(md.render(imageMarkdown), /<img src="\.\.\/assets\/p\.png" alt="截图" width="320">/);
for (const tag of ["<img src='pic.png' alt='hello' />", '<IMG SRC=pic.png alt=hello>', '<img src="pic.png" alt="hello">']) {
  assert.match(md.render(`before ${tag} after`), /before <img src="pic.png" alt="hello"> after/);
}
assert.match(md.render('<img src="p.png" title="a &amp; b" width=50% height=120 onerror="alert(1)" style="display:none" class=x>'), /<img src="p.png" alt="" title="a &amp; b" width="50%" height="120">/);
for (const src of ['javascript:alert(1)', 'JaVaScRiPt:alert(1)', 'jav&#x61;script:alert(1)', 'data:text/html,x', 'file:///tmp/x']) {
  assert.doesNotMatch(md.render(`<img src="${src}">`), /<img /);
}
for (const literal of ['`<img src=x>`', '```html\n<img src=x>\n```', '    <img src=x>', '<!-- <img src=x> -->']) {
  assert.doesNotMatch(md.render(literal), /<img /);
  assert.match(md.render(literal), /&lt;img/);
}
assert.doesNotMatch(md.render('<script>alert(1)</script><iframe src=x></iframe>'), /<(script|iframe)/);
// Exercise the actual share image renderer and resource resolver without starting its DOM app.
const mainSource = await readFile(new URL('../src/share/main.ts', import.meta.url), 'utf8');
const rendererSource = mainSource.slice(mainSource.indexOf('md.renderer.rules.image ='), mainSource.indexOf('md.renderer.rules.link_open ='));
const resourceSource = mainSource.slice(mainSource.indexOf('function resourceUrl('), mainSource.indexOf('function findEntry('));
const integrationCode = await transform(`export function install(md, safeHref, resolveVaultPath, encodedPath, fileUrl, app) { md.validateLink = (url) => safeHref(url) !== null; ${resourceSource}\n${rendererSource}\n}`, { loader: 'ts', format: 'esm' });
const { install } = await import(`data:text/javascript;base64,${Buffer.from(integrationCode.code).toString('base64')}`);
const shareFileUrl = (path) => `/files/${encodedPath(path)}`;
const shareMd = new MarkdownIt({ html: false }).use(centerContainers).use(htmlImages);
const imageCatalog = { catalog: { entries: [{ path: 'assets/paste.png', kind: 'image' }, { path: 'notes/assets/local.png', kind: 'image' }, { path: 'assets/local.png', kind: 'image' }] } };
install(shareMd, safeHref, resolveVaultPath, encodedPath, shareFileUrl, imageCatalog);
assert.match(shareMd.render('![](assets/paste.png)', { path: 'notes/doc.md' }), /src="\/files\/assets\/paste.png"/);
assert.match(shareMd.render('![](assets/local.png)', { path: 'notes/doc.md' }), /src="\/files\/notes\/assets\/local.png"/);
assert.match(shareMd.render('![](/assets/a%20b.png)', { path: 'notes/doc.md' }), /src="\/files\/assets\/a%20b.png"/);
const env = { path: 'notes/doc.md' };
assert.equal(shareMd.render('<img src="../assets/a b.png" alt="截图">', env).match(/src="([^"]+)"/)[1], shareMd.render('![截图](<../assets/a b.png>)', env).match(/src="([^"]+)"/)[1]);
assert.match(shareMd.render(imageMarkdown, env), /src="\/files\/assets\/p.png" alt="截图" width="320" loading="lazy"/);
assert.match(shareMd.render('<img src="https://example.com/p.png">', env), /src="https:\/\/example.com\/p.png"/);
const shareHtml = await readFile(new URL('../src/share/index.html', import.meta.url), 'utf8');
assert.ok(shareHtml.includes('href="share.css"'));
assert.ok(shareHtml.includes('src="share.js"'));
assert.ok(mainSource.includes("app.static ? 'catalog.json' : '/api/catalog'"));
assert.ok(mainSource.includes('function routeUrl(path: string)'));
console.log('share utility, center, code block, and HTML image rendering tests passed');

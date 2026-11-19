import assert from 'node:assert/strict';
import { build } from 'esbuild';
import { readFile, mkdtemp, rm, access } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createServer } from 'node:http';
import { spawn } from 'node:child_process';

const root = new URL('../', import.meta.url);
const i18nBuild = await build({ entryPoints: [new URL('src/share/i18n.ts', root).pathname], bundle: true, platform: 'node', format: 'esm', write: false });
const { detectLanguage, setLanguage, t } = await import(`data:text/javascript;base64,${Buffer.from(i18nBuild.outputFiles[0].contents).toString('base64')}`);
for (const [saved, browser, expected] of [
  ['en', ['zh-CN'], 'en'], ['zh-CN', ['en-US'], 'zh-CN'], [null, ['en-GB'], 'en'],
  [null, ['zh-TW'], 'zh-CN'], ['invalid', ['fr', 'en-US'], 'en'], [null, ['fr', 'zh-CN'], 'zh-CN'], [null, [], 'en'],
]) assert.equal(detectLanguage(saved, browser), expected);
setLanguage('en'); assert.equal(t('updated', { date: 'DATE' }), 'Updated: DATE');
assert.equal(t('readError', { error: '<unknown>' }), 'Failed to read: <unknown>');
setLanguage('zh-CN'); assert.equal(t('copyCode'), '复制代码');

const browser = process.env.SHARE_TEST_BROWSER || '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome';
try { await access(browser); } catch {
  console.log('share i18n language detection tests passed; browser checks skipped (set SHARE_TEST_BROWSER to Chrome/Chromium)');
  process.exit(0);
}
const site = await readFile(new URL('src/share/index.html', root), 'utf8');
const login = await readFile(new URL('src/share/login.html', root), 'utf8');
const loginScript = await readFile(new URL('src/share/login.js', root), 'utf8');
const rustSource = await readFile(new URL('src-tauri/src/share.rs', root), 'utf8');
const csp = rustSource.match(/"(default-src 'self'; script-src 'self';[^"\n]+)"/)[1];
assert.doesNotMatch(login, /<script>/);
assert.match(login, /<script src="\/login.js"><\/script>/);
assert.equal((login.match(/\{\{error\}\}/g) || []).length, 1);
assert.match(login, /<form action="\/login" method="post">/);
assert.match(login, /name="password" type="password"/);
const css = `${await readFile(new URL('src/assets/theme.css', root), 'utf8')}\n${await readFile(new URL('src/share/site.css', root), 'utf8')}`;
const bundle = await build({ entryPoints: [new URL('src/share/main.ts', root).pathname], bundle: true, platform: 'browser', format: 'iife', write: false, loader: { '.css': 'empty' } });
let noteRequests = 0;
const harness = `<!doctype html><html><body><pre id="result">RUNNING</pre><iframe id="site"></iframe><iframe id="login"></iframe><script>
(async () => {
 const assert = (value, message) => { if (!value) throw new Error(message); };
 const wait = async predicate => { for (let i = 0; i < 200; i++) { if (predicate()) return; await new Promise(resolve => setTimeout(resolve, 20)); } throw new Error('Timed out'); };
 const load = (frame, url) => new Promise(resolve => { frame.onload = resolve; frame.src = url; });
 const result = document.getElementById('result');
 try {
  localStorage.setItem('share-language', 'en');
  const frame = document.getElementById('site');
  await load(frame, '/?static#Folder%2FNote.md');
  const doc = frame.contentDocument;
  await wait(() => doc.querySelector('.code-copy'));
  assert(doc.documentElement.lang === 'en', 'Saved English preference');
  assert(doc.title === 'Shared vault', 'English title');
  assert(doc.querySelector('.download').textContent === 'Download', 'English download');
  assert(doc.querySelector('.wiki-unresolved').title === 'Note not found', 'English wiki hint');
  assert(doc.querySelector('.code-copy').getAttribute('aria-label') === 'Copy code', 'English code label');
  assert(doc.querySelector('time').textContent.startsWith('Updated: '), 'English date');
  assert(doc.querySelector('#vault-name').textContent === '用户知识库', 'Vault name preserved');
  doc.querySelector('#reading-mode').click();
  const body = doc.querySelector('.file-body');
  const treeRow = doc.querySelector('.tree-row.active');
  const picker = doc.querySelector('#language-select');
  picker.value = 'zh-CN'; picker.dispatchEvent(new frame.contentWindow.Event('change'));
  assert(doc.documentElement.lang === 'zh-CN', 'Chinese switch');
  assert(doc.querySelector('.file-body') === body, 'Content DOM preserved');
  assert(doc.querySelector('.tree-row.active') === treeRow, 'Tree DOM preserved');
  assert(doc.querySelector('[aria-expanded="true"]'), 'Expanded folder preserved');
  assert(doc.body.classList.contains('reading-mode'), 'Reading mode preserved');
  assert(doc.querySelector('#reading-mode').textContent === '退出阅读', 'Reading label updated');
  assert(doc.querySelector('.code-copy').title === '复制代码', 'Code hint updated');
  assert(doc.querySelector('.wiki-unresolved').title === '未找到笔记', 'Wiki hint updated');
  assert(doc.querySelector('.download').textContent === '下载', 'Download updated');
  assert(doc.querySelector('time').textContent.startsWith('更新时间：'), 'Date updated');
  assert(body.textContent.includes('用户内容 User content'), 'User content preserved');
  assert(frame.contentWindow.location.hash === '#Folder%2FNote.md', 'Selected file preserved');
  // Simulate blocked clipboard and verify both active feedback and timer reset use the current language.
  Object.defineProperty(frame.contentWindow.navigator, 'clipboard', { configurable: true, value: { writeText: async () => { throw new Error('blocked'); } } });
  doc.execCommand = () => false;
  doc.querySelector('.code-copy').click();
  await wait(() => doc.querySelector('.code-copy').title === '复制失败');
  picker.value = 'en'; picker.dispatchEvent(new frame.contentWindow.Event('change'));
  assert(doc.querySelector('.code-copy').title === 'Copy failed', 'Active feedback translates');
  await wait(() => doc.querySelector('.code-copy').title === 'Copy code');
  assert(doc.querySelector('.code-copy').getAttribute('aria-label') === 'Copy code', 'Feedback resets to current language');
  const loginFrame = document.getElementById('login');
  await load(loginFrame, '/login-known');
  const ld = loginFrame.contentDocument;
  assert(ld.documentElement.lang === 'en', 'Login uses preference with same-origin script under CSP');
  assert(ld.querySelector('#login-error').textContent === 'Incorrect password. Please try again.', 'Known server error translates');
  assert(ld.querySelector('form').getAttribute('action') === '/login', 'Login action preserved');
  assert(ld.querySelector('form').method === 'post', 'Login POST preserved');
  const password = ld.querySelector('[name="password"]'); password.value = 'keep-secret';
  const lp = ld.querySelector('select'); lp.value = 'zh-CN'; lp.dispatchEvent(new loginFrame.contentWindow.Event('change'));
  assert(password.value === 'keep-secret', 'Password preserved when switching');
  assert(ld.querySelector('#login-error').textContent === '密码错误，请重试', 'Known error switches back');
  localStorage.setItem('share-language', 'en');
  await load(loginFrame, '/login-unknown');
  assert(loginFrame.contentDocument.querySelector('#login-error').textContent === '后端未知错误', 'Unknown server error preserved');
  const originalStorage = Object.getOwnPropertyDescriptor(frame.contentWindow, 'localStorage');
  Object.defineProperty(frame.contentWindow, 'localStorage', { configurable: true, get() { throw new Error('blocked'); } });
  picker.value = 'zh-CN'; picker.dispatchEvent(new frame.contentWindow.Event('change'));
  assert(doc.documentElement.lang === 'zh-CN', 'Switch works with blocked storage');
  if (originalStorage) Object.defineProperty(frame.contentWindow, 'localStorage', originalStorage);
  result.textContent = 'PASS';
 } catch (error) { result.textContent = 'FAIL: ' + error.message; }
})();
</script></body></html>`;
const server = createServer((request, response) => {
  const pathname = new URL(request.url, 'http://localhost').pathname;
  let body; let type = 'text/html; charset=utf-8';
  if (pathname === '/test') body = harness;
  else if (pathname.startsWith('/login-')) body = login.replace('{{error}}', pathname === '/login-known' || pathname === '/login-csp' ? '密码错误，请重试' : '后端未知错误');
  else if (pathname === '/login.js') { body = loginScript; type = 'text/javascript'; }
  else if (pathname === '/') body = site;
  else if (pathname === '/share.js') { body = bundle.outputFiles[0].contents; type = 'text/javascript'; }
  else if (pathname === '/share.css') { body = css; type = 'text/css'; }
  else if (pathname === '/catalog.json') {
    body = JSON.stringify({ name: '用户知识库', stats: { files: 1, folders: 1, notes: 1, bytes: 100 }, entries: [{ path: 'Folder', kind: 'folder' }, { path: 'Folder/Note.md', kind: 'markdown', modified: 1700000000 }] }); type = 'application/json';
  } else if (pathname === '/files/Folder/Note.md') { noteRequests++; body = '# 用户内容 User content\n\n[[missing]]\n\n```js\nconst value = 1;\n```'; type = 'text/plain'; }
  else { response.writeHead(404); response.end(); return; }
  // The login frame needs to be inspected by this same-origin harness; retain the exact production script policy.
  const headers = { 'Content-Type': type };
  if (pathname !== '/test') headers['Content-Security-Policy'] = pathname === '/login-csp' ? csp : csp.replace("frame-ancestors 'none'", "frame-ancestors 'self'");
  response.writeHead(200, headers); response.end(body);
});
await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
const profile = await mkdtemp(path.join(tmpdir(), 'easymd-share-i18n-'));
try {
  const { port } = server.address();
  const child = spawn(browser, ['--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check', `--user-data-dir=${profile}`, '--virtual-time-budget=12000', '--dump-dom', `http://127.0.0.1:${port}/test`]);
  let output = ''; let errors = '';
  child.stdout.on('data', chunk => { output += chunk; }); child.stderr.on('data', chunk => { errors += chunk; });
  const timer = setTimeout(() => child.kill('SIGKILL'), 45000);
  const code = await new Promise((resolve, reject) => { child.on('error', reject); child.on('close', resolve); });
  clearTimeout(timer);
  assert.equal(code, 0, errors.slice(-2000));
  assert.match(output, /<pre id="result">PASS<\/pre>/, output.match(/<pre id="result">[^<]*/)?.[0] || errors.slice(-2000));
  assert.equal(noteRequests, 1, 'Language switches must not refetch the note');
  const cspChild = spawn(browser, ['--headless', '--disable-gpu', '--no-first-run', '--no-default-browser-check', '--lang=en-US', `--user-data-dir=${profile}`, '--virtual-time-budget=3000', '--dump-dom', `http://127.0.0.1:${port}/login-csp`]);
  let cspOutput = ''; let cspErrors = '';
  cspChild.stdout.on('data', chunk => { cspOutput += chunk; }); cspChild.stderr.on('data', chunk => { cspErrors += chunk; });
  const cspTimer = setTimeout(() => cspChild.kill('SIGKILL'), 45000);
  const cspCode = await new Promise((resolve, reject) => { cspChild.on('error', reject); cspChild.on('close', resolve); });
  clearTimeout(cspTimer);
  assert.equal(cspCode, 0, cspErrors.slice(-2000));
  assert.match(cspOutput, /<html lang="en"/, 'Login script must execute under the exact production CSP');
  assert.match(cspOutput, /Incorrect password\. Please try again\./);
  console.log('share i18n detection and Chrome integration tests passed (static relative routes, live language/state, copy feedback, login safety, exact production CSP)');
} finally { server.close(); await rm(profile, { recursive: true, force: true }); }

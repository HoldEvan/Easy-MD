import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
import * as vue from 'vue';

const source = readFileSync(new URL('../src/composables/useImageInsertion.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
const dictionarySource = readFileSync(new URL('../src/i18n/imageUpload.ts', import.meta.url), 'utf8');
const dictionaryExports = {};
vm.runInNewContext(ts.transpileModule(dictionarySource, { compilerOptions: { module: ts.ModuleKind.CommonJS } }).outputText, { exports: dictionaryExports });
const dictionary = dictionaryExports.imageUploadMessages;
function setup(provider = 's3') {
  const mounts = [], unmounts = [], inserted = [], calls = [];
  const editor = vue.reactive({ activePath: 'a.md', openToken: 1 });
  const vault = vue.reactive({ root: '/vault', opened: true });
  const behavior = { provider, failure: false, upload: null, configFailure: false };
  const locale = vue.ref('zh');
  const t = key => dictionary[key]?.[locale.value === 'en' ? 1 : 0] ?? key;
  const tf = (key, vars) => Object.entries(vars).reduce((text, [name, value]) => text.replaceAll(`{${name}}`, String(value)), t(key));
  const mocks = {
    '../i18n': { t, tf },
    vue: { ...vue, onMounted: f => mounts.push(f), onUnmounted: f => unmounts.push(f) },
    '@tauri-apps/api/core': { isTauri: () => false },
    '@tauri-apps/api/webview': {},
    '@tauri-apps/plugin-dialog': { open: async () => null },
    '../stores/editor': { useEditorStore: () => editor },
    '../stores/vault': { useVaultStore: () => vault },
    '../ipc/tauri': { api: { saveImage: async (...args) => { calls.push(['local', ...args]); return 'assets/local.png'; }, readExternalBinary: async () => [1, 2] } },
    '../lib/imageUploadApi': {
      getImageUploadConfig: async () => { calls.push(['config']); if (behavior.configFailure) throw Error('config unavailable'); return { provider: behavior.provider }; },
      uploadImage: async (...args) => { calls.push(['upload', ...args]); if (behavior.failure) throw Error('upload failed'); return behavior.upload ? behavior.upload() : { url: 'https://example.test/image.png' }; },
    },
  };
  const exports = {};
  vm.runInNewContext(compiled, { exports, require: name => mocks[name], Uint8Array, Map, Error, String, Array });
  const queue = exports.useImageInsertion(vue.ref(), text => inserted.push(text));
  mounts.forEach(f => f());
  function paste(size = 2) {
    const file = { name: 'a.png', type: 'image/png', size, arrayBuffer: async () => new Uint8Array([1, 2]).buffer };
    queue.paste({ clipboardData: { items: [{ type: file.type, getAsFile: () => file }] }, preventDefault() {}, stopPropagation() {} });
  }
  return { queue, paste, behavior, calls, inserted, editor, vault, unmounts, locale, tf };
}
async function settle() { for (let i = 0; i < 15; i++) await Promise.resolve(); }
{
  const s = setup('local'); s.paste(); await settle();
  assert.equal(s.inserted.length, 1); assert.equal(s.calls.filter(c => c[0] === 'upload').length, 0);
  assert.equal(s.calls.find(c => c[0] === 'local')[3], '/vault');
}
for (const provider of ['s3', 'picgo']) {
  const s = setup(provider); s.paste(); await settle();
  assert.equal(s.inserted[0], '![](<https://example.test/image.png>)\n');
}
{
  const s = setup(); s.behavior.failure = true; s.paste(); await settle();
  assert.equal(s.queue.jobs[0].state, 'failed'); assert.equal(s.inserted.length, 0);
  s.behavior.failure = false; s.queue.retry(s.queue.jobs[0].id); await settle();
  assert.equal(s.inserted.length, 1); assert.equal(s.calls.filter(c => c[0] === 'config').length, 2);
}
{
  const s = setup(); s.behavior.configFailure = true; s.paste(); await settle();
  s.queue.retry(s.queue.jobs[0].id, true); await settle();
  assert.equal(s.inserted[0], '![](<assets/local.png>)\n');
}
{
  const s = setup(); let resolve;
  s.behavior.upload = () => new Promise(r => { resolve = r; }); s.paste(); await settle();
  s.editor.activePath = 'b.md'; s.editor.openToken++;
  resolve({ url: 'https://example.test/done.png' }); await settle();
  assert.equal(s.inserted.length, 0); assert.equal(s.queue.jobs[0].state, 'pending');
  s.queue.retry(s.queue.jobs[0].id); await settle(); assert.equal(s.inserted.length, 0);
  s.editor.activePath = 'a.md'; s.editor.openToken++; s.queue.retry(s.queue.jobs[0].id); await settle();
  assert.equal(s.inserted.length, 1); assert.equal(s.calls.filter(c => c[0] === 'upload').length, 1);
}
{
  const s = setup(); let resolve;
  s.behavior.upload = () => new Promise(r => { resolve = r; }); s.paste(); await settle();
  s.queue.cancel(s.queue.jobs[0].id); resolve({ url: 'https://example.test/cancelled.png' }); await settle();
  assert.equal(s.inserted.length, 0); assert.equal(s.queue.jobs.length, 0);
}
{
  const s = setup(); s.paste(20 * 1024 * 1024 + 1); await settle();
  assert.equal(s.queue.jobs[0].state, 'failed'); assert.equal(s.calls.length, 0);
}
{
  const s = setup(); s.paste(); s.vault.root = '/other'; await settle();
  assert.equal(s.inserted.length, 0); assert.equal(s.calls.filter(c => c[0] === 'upload' || c[0] === 'local').length, 0);
}
{
  const s = setup(); s.editor.activePath = ''; await s.queue.choose();
  assert.equal(s.queue.notice.value, '请先打开一篇笔记再插入图片');
  s.locale.value = 'en'; assert.equal(s.queue.notice.value, 'Open a note before inserting images');
  s.queue.notice.value = ''; assert.equal(s.queue.notice.value, '');
}
{
  const s = setup(); s.paste(20 * 1024 * 1024 + 1); await settle();
  const job = s.queue.jobs[0];
  assert.equal(s.tf(job.messageKey, job.messageVars), '图片超过 20 MiB，请缩小后重新选择');
  s.locale.value = 'en';
  assert.equal(s.tf(job.messageKey, job.messageVars), 'The image exceeds 20 MiB. Reduce its size and select it again');
}
{
  const s = setup(); s.behavior.failure = true; s.paste(); await settle();
  assert.equal(s.queue.jobs[0].messageKey, undefined);
  assert.equal(s.queue.jobs[0].message, 'upload failed');
  s.locale.value = 'en'; assert.equal(s.queue.jobs[0].message, 'upload failed');
}
{
  const s = setup('local'); s.paste(); await settle();
  assert.equal(s.queue.notice.value, '已插入图片：a.png');
  s.locale.value = 'en'; assert.equal(s.queue.notice.value, 'Image inserted: a.png');
}
console.log('Image insertion checks passed: local/S3/PicGo, retry, local fallback, note/vault switch, cancel, 20MiB cap, live language switching, raw error preservation.');

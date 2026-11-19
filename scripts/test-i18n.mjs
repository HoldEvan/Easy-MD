import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import ts from 'typescript';
import * as vue from 'vue';
import vm from 'node:vm';

// Load the real dictionaries and translator without a browser or Tauri runtime.
const modules = new Map();
function load(path) {
  if (modules.has(path)) return modules.get(path);
  const source = readFileSync(new URL(`../src/${path}.ts`, import.meta.url), 'utf8');
  const exports = {};
  const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  vm.runInNewContext(compiled, { exports, require: name => {
    if (name === 'vue') return vue;
    if (name === '../lib/platform') return { platformText: s => s };
    return load(`i18n/${name.slice(2)}`);
  }, document: { documentElement: { dataset: {} } } });
  modules.set(path, exports);
  return exports;
}
const i18n = load('i18n/index');
let total = 0;
const seen = new Set();
for (const [path, symbol] of [['share', 'shareMessages'], ['graph', 'graphMessages'], ['imageUpload', 'imageUploadMessages']]) {
  const messages = load(`i18n/${path}`)[symbol];
  for (const [key, values] of Object.entries(messages)) {
    assert.ok(!seen.has(key), `Duplicate translation key: ${key}`); seen.add(key);
    assert.equal(values.length, 2, key);
    assert.ok(values[0].trim() && values[1].trim(), `Empty translation: ${key}`);
    assert.ok(!/[\p{Script=Han}]/u.test(values[1]), `Chinese in English translation: ${key}`);
    assert.deepEqual([...values[0].matchAll(/\{(\w+)\}/g)].map(m=>m[1]).sort(), [...values[1].matchAll(/\{(\w+)\}/g)].map(m=>m[1]).sort(), `Placeholder mismatch: ${key}`);
    for (const [index, lang] of ['zh','en'].entries()) {
      i18n.setLang(lang); assert.equal(i18n.t(key), values[index], `Dictionary not registered: ${key}`);
    }
    total++;
  }
}
function files(dir) { return readdirSync(new URL(`../${dir}`, import.meta.url), { withFileTypes: true }).flatMap(e=>e.isDirectory()?files(`${dir}/${e.name}`): /\.(vue|ts)$/.test(e.name)?[`${dir}/${e.name}`]:[]); }
const missing = [];
for (const path of files('src/components').concat(files('src/composables'), files('src/lib/markdown'))) {
  const source = readFileSync(new URL(`../${path}`, import.meta.url), 'utf8');
  for (const [, key] of source.matchAll(/\b(?:t|tf)\(\s*["']([^"']+)["']/g)) {
    if (i18n.t(key) === key) missing.push(`${key} in ${path}`);
  }
}
assert.deepEqual(missing, [], `Missing translations: ${missing.join(', ')}`);
i18n.setLang('en');assert.equal(i18n.tf('graph.locateCount',{n:3}), 'Locate (3)');
i18n.setLang('zh');assert.equal(i18n.tf('graph.locateCount',{n:3}), '定位（3）');
console.log(`PASS ${total} new bilingual entries, placeholders, dictionary registration and component translation references`);

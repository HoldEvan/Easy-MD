import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import ts from 'typescript';
import * as vue from 'vue';
const source = readFileSync('src/components/sidebar/FileTree.vue', 'utf8').split('<script setup lang="ts">')[1].split('</script>')[0];
const ui = vue.reactive({ view: 'editor', imagePath: '', rightVisible: true });
let opened = '', saves = 0;
const editor = { activePath: 'note.md', isDirty: false, save: async () => { saves++; } };
const imports = {
  vue: { ...vue, watch: () => {} },
  '../common/Icon.vue': {},
  '../../stores/vault': { useVaultStore: () => ({}) },
  '../../stores/ui': { useUiStore: () => ui },
  '../../stores/editor': { useEditorStore: () => editor },
  '../../stores/settings': { useSettingsStore: () => ({}) },
};
globalThis.window = { addEventListener() {} };
globalThis.defineProps = () => ({ dir: '' });
globalThis.withDefaults = p => p;
globalThis.defineEmits = () => (event, path) => { if (event === 'open') opened = path; };
const js = ts.transpileModule(source + '\nexport { openEntry, isActive };', { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
const component = {};
Function('require', 'exports', js)(name => imports[name], component);
const image = { path: 'assets/pic.png', kind: 'image' };
await component.openEntry(image);
assert.equal(opened, '', 'Image must not enter openNote/readTextFile');
assert.equal(ui.view, 'image');
assert.equal(ui.imagePath, image.path);
assert.equal(component.isActive(image), true);
assert.equal(ui.rightVisible, true, 'Keep the note sidebar preference');
editor.isDirty = true;
await component.openEntry({ path: 'assets/next.png', kind: 'image' });
assert.equal(saves, 1, 'Save pending note before changing view');
await component.openEntry({ path: 'note.md', kind: 'md' });
assert.equal(ui.view, 'editor');
assert.equal(opened, 'note.md');
console.log('PASS image selection bypasses text loading, saves pending note and preserves sidebar preference');

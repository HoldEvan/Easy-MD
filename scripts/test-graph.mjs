import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import ts from 'typescript';
import * as vue from 'vue';
const require = createRequire(import.meta.url);
function evaluate(source, imports) {
  const js = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  const exports = {};
  Function('require', 'exports', js)((name) => name in imports ? imports[name] : require(name), exports);
  return exports;
}
const links = evaluate(readFileSync('src/lib/markdown/links.ts', 'utf8'), {});
const graph = evaluate(readFileSync('src/lib/graph/data.ts', 'utf8'), { '../markdown/links': links });
const note = (path, targets = []) => ({ path, title: path.split('/').pop().replace(/\.md$/, ''), aliases: [], links: targets.map(target => ({ target })) });
const notes = [note('a.md', ['b', 'b', 'image.png', 'file.pdf', 'missing', 'same']), note('b.md', ['a', 'c']), note('c.md', ['d']), note('d.md'), note('x/same.md'), note('y/same.md')];
const data = graph.buildAutoGraph(notes, true);
assert.equal(data.links.length, 4, 'deduplicate repeated and reverse edges; ignore ambiguous targets/assets');
assert.deepEqual(data.nodes.filter(n => n.unresolved).map(n => n.title), ['missing']);
assert.deepEqual(graph.localGraph(data, 'a.md', 1).nodes.map(n => n.id).sort(), ['?missing', 'a.md', 'b.md']);
assert.equal(graph.localGraph(data, 'a.md', 2).nodes.some(n => n.id === 'c.md'), true);
assert.equal(graph.localGraph(data, 'a.md', 3).nodes.some(n => n.id === 'd.md'), true);
assert.equal(graph.localGraph(data, 'd.md', 1).nodes.some(n => n.id === 'c.md'), true, 'incoming references are neighbors');
assert.equal(graph.buildAutoGraph(notes, false).nodes.some(n => n.unresolved), false);
const relative = graph.buildAutoGraph([note('folder/a.md', ['./b.md']), note('folder/b.md')], true);
assert.equal(relative.links[0].target, 'folder/b.md', 'pass source path to resolver');

const callbacks = {};
let live = { nodes: [], links: [] };
const fg = new Proxy({}, { get: (_, key) => {
  assert.notEqual(key, 'onEdgeClick', 'force-graph does not implement onEdgeClick');
  return (...args) => {
    if (key === 'graphData') { if (!args.length) return live; live = args[0]; }
    callbacks[key] = args[0];
    return fg;
  };
} });
const ui = vue.reactive({ graphPath: '', view: 'graph' });
const pendingReads = new Map();
const writes = [];
let opened = '';
const api = { readTextFile: path => new Promise(resolve => pendingReads.set(path, resolve)), writeTextFile: async (path, text) => { writes.push({ path, text }); }, listAllFilesWithMtime: async () => [] };
const source = readFileSync('src/components/graph/GraphView.vue', 'utf8').split('<script setup lang="ts">')[1].split('</script>')[0];
globalThis.document = { documentElement: {} };
globalThis.getComputedStyle = () => ({ getPropertyValue: () => '#888888' });
globalThis.confirm = () => true;
const component = evaluate(source + '\nexport { render, container, doc, loadDoc, editMode, selectedId, onSelectGraph, flushSave, scheduleSave, toggleEdit, onResize, searchQuery, locateNode, fitGraph };', {
  vue: { ...vue, onMounted: () => {}, onBeforeUnmount: () => {} },
  'force-graph': { default: function () { return fg; } },
  '../../stores/notesIndex': { useNotesIndexStore: () => vue.reactive({ notes }) },
  '../../stores/editor': { useEditorStore: () => ({ activePath: 'a.md', openNote: async path => { opened = path; } }) },
  '../../stores/ui': { useUiStore: () => ui },
  '../../stores/settings': { useSettingsStore: () => ({}) },
  '../../lib/graph/data': graph,
  '../../lib/markdown/links': links,
  '../../i18n': { t: x => x, tf: x => x },
  '../common/Icon.vue': {}, '../common/InputDialog.vue': {}, '../common/DropdownMenu.vue': {},
  '../../ipc/tauri': { api },
});
component.container.value = { clientWidth: 640, clientHeight: 480 };
component.render();
assert.equal(callbacks.width, 640);
assert.equal(callbacks.height, 480);
component.container.value.clientWidth = 720;
component.onResize();
assert.equal(callbacks.width, 720);
live.nodes[0].x = 25;
live.nodes[0].y = 30;
component.searchQuery.value = 'a.md';
component.locateNode();
assert.equal(callbacks.centerAt, 25);
assert.equal(callbacks.zoom, 2);
component.fitGraph();
assert.equal(callbacks.zoomToFit, 300);
const click = callbacks.onNodeClick;
const tick = async () => { await Promise.resolve(); await Promise.resolve(); await vue.nextTick(); };
const doc = label => JSON.stringify({ nodes: [{ id: 'n', label, note: 'a.md' }], edges: [] });
component.onSelectGraph('first.graph');
await tick();
component.onSelectGraph('second.graph');
await tick();
pendingReads.get('second.graph')(doc('second'));
await tick();
pendingReads.get('first.graph')(doc('first'));
await tick();
assert.equal(component.doc.value.nodes[0].label, 'second', 'late load cannot replace current graph');
component.editMode.value = true;
click({ id: 'n', note: 'a.md' });
assert.equal(component.selectedId.value, 'n', 'initial callback reads current manual mode');
assert.equal(opened, '', 'editing a bound node selects without navigating');
component.doc.value.nodes[0].label = 'changed';
component.scheduleSave();
component.onSelectGraph('third.graph');
await component.flushSave();
assert.equal(writes[0].path, 'second.graph');
assert.equal(JSON.parse(writes[0].text).nodes[0].label, 'changed');
await tick();
pendingReads.get('third.graph')('invalid JSON');
await tick();
assert.equal(component.doc.value, null, 'failed load never creates writable empty document');
await component.flushSave();
assert.equal(writes.length, 1);
console.log('Graph regressions passed: data resolution/dedup/assets, 1–3 layers, initial sizing, dynamic clicks, edit selection, stale reads, captured saves, corrupt files.');

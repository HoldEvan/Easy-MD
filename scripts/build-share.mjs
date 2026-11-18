import { build } from 'esbuild';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const shareDir = path.join(root, 'src/share');
const result = await build({
  entryPoints: [path.join(shareDir, 'main.ts')],
  bundle: true,
  platform: 'browser',
  format: 'iife',
  target: ['es2020'],
  minify: true,
  write: false,
  loader: { '.css': 'empty' },
  logLevel: 'info'
});
const js = result.outputFiles[0];
if (!js) throw new Error('esbuild did not produce JavaScript');
await mkdir(shareDir, { recursive: true });
await writeFile(path.join(shareDir, 'share.js'), js.contents);
const theme = await readFile(path.join(root, 'src/assets/theme.css'), 'utf8');
const site = await readFile(path.join(shareDir, 'site.css'), 'utf8');
await writeFile(path.join(shareDir, 'share.css'), `${theme}\n${site}`);
console.log('Built src/share/share.js and inlined src/assets/theme.css into src/share/share.css');

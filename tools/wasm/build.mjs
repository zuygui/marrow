// Builds what the in-browser playground needs, from a Marrow compiler binary that knows
// `--emit-il`:
//   node tools/wasm/build.mjs <marrow-binary> <out-dir>
// Writes compiler.wasm (the compiler itself, translated from its own IL), compiler.json (the
// addresses the host needs), std.json (the standard library sources) and copies the
// translator and the runtime next to them.
import fs from 'fs';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import { execFileSync } from 'child_process';
import { translate } from './il2wasm.mjs';

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, '..', '..');
const [marrow, out] = process.argv.slice(2);
if (!marrow || !out) { console.error('usage: node build.mjs <marrow-binary> <out-dir>'); process.exit(2); }
fs.mkdirSync(out, { recursive: true });

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'marrow-pg-'));
const il = path.join(tmp, 'compiler.ssa');
execFileSync(marrow, ['--emit-il', '--nostdlib', '--color', 'never', '--target', 'x86_64-linux',
  path.join(root, 'compiler', 'marrow.mw'), il], { stdio: 'inherit' });
const t = translate(fs.readFileSync(il, 'utf8'), { stackSize: 16 * 1024 * 1024 });
fs.writeFileSync(path.join(out, 'compiler.wasm'), t.wasm);
fs.writeFileSync(path.join(out, 'compiler.json'), JSON.stringify({ symbols: { SYS_ENVP: t.symbols.SYS_ENVP }, layout: t.layout }));

const std = {};
(function walk(dir, rel) {
  for (const e of fs.readdirSync(dir, { withFileTypes: true })) {
    const p = path.join(dir, e.name), r = rel + e.name;
    if (e.isDirectory()) walk(p, r + '/');
    else if (e.name.endsWith('.mw')) std[r] = fs.readFileSync(p, 'utf8');
  }
})(path.join(root, 'std'), 'std/');
fs.writeFileSync(path.join(out, 'std.json'), JSON.stringify(std));

for (const f of ['il2wasm.mjs', 'runtime.mjs']) fs.copyFileSync(path.join(here, f), path.join(out, f));
fs.rmSync(tmp, { recursive: true, force: true });
console.log(`compiler.wasm ${t.wasm.length} bytes, std.json ${Object.keys(std).length} files`);

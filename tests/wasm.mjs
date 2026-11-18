// Runs the programs of tests/run in WebAssembly: `marrow --emit-il` writes the IL, tools/wasm
// translates it and runs it over emulated system calls. Needs node and a built compiler:
//   node tests/wasm.mjs [filter]          (the compiler is build/marrow, or $MARROW)
// Programs that need threads, processes, sockets or the system certificates cannot run in a
// browser and are skipped.
import fs from 'fs';
import os from 'os';
import path from 'path';
import { fileURLToPath } from 'url';
import { execFileSync } from 'child_process';
import { translate } from '../tools/wasm/il2wasm.mjs';
import { Process } from '../tools/wasm/runtime.mjs';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const marrow = process.env.MARROW || path.join(root, 'build', 'marrow');
const dir = path.join(root, 'tests', 'run');
const skip = new Set(['core', 'http', 'net', 'task', 'task_net', 'sysenv', 'x509', 'modules', 'semver_toml']);
const only = process.argv[2];
const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'marrow-wasm-'));
let ok = 0;
const fails = [];
for (const f of fs.readdirSync(dir).filter(f => f.endsWith('.mw')).sort()) {
  const name = f.slice(0, -3);
  if (skip.has(name) || (only && !name.includes(only))) continue;
  const expFile = path.join(dir, name + '.stdout');
  if (!fs.existsSync(expFile)) continue;
  const work = path.join(tmp, name);
  fs.mkdirSync(work, { recursive: true });
  fs.cpSync(path.join(root, 'std'), path.join(work, 'std'), { recursive: true });
  fs.copyFileSync(path.join(dir, f), path.join(work, f));
  try {
    execFileSync(marrow, ['--emit-il', '--nostdlib', '--color', 'never', '--target', 'x86_64-linux', f, name + '.ssa'], { cwd: work, stdio: 'pipe' });
  } catch (e) { fails.push(`${name}: compile: ${String(e.stderr).split('\n')[0]}`); continue; }
  const out = [];
  let note = '';
  try {
    const t = translate(fs.readFileSync(path.join(work, name + '.ssa'), 'utf8'));
    const p = new Process({ stdout: b => out.push(Buffer.from(b)), stderr: () => {}, log: m => { note += m + '; '; }, cwd: '/' });
    await p.run(t.wasm, t.symbols, t.layout);
  } catch (e) { note += 'trap: ' + e.message; }
  if (Buffer.concat(out).equals(fs.readFileSync(expFile))) ok++;
  else fails.push(`${name}: output differs ${note}`);
}
fs.rmSync(tmp, { recursive: true, force: true });
console.log(`wasm: ${ok} passed, ${fails.length} failed`);
for (const f of fails) console.log('  FAIL ' + f);
process.exit(fails.length ? 1 : 0);

// Runs the Marrow compiler (itself compiled to WebAssembly) on one source file and answers
// with the QBE IL and the diagnostics. The module is compiled once and kept.
import { Process, Vfs } from './runtime.mjs';

let ready = null;
async function init() {
  const wasm = fetch('compiler.wasm');
  const [mod, meta, std] = await Promise.all([
    WebAssembly.compileStreaming ? WebAssembly.compileStreaming(wasm).catch(async () => WebAssembly.compile(await (await fetch('compiler.wasm')).arrayBuffer()))
      : wasm.then(r => r.arrayBuffer()).then(b => WebAssembly.compile(b)),
    fetch('compiler.json').then(r => r.json()),
    fetch('std.json').then(r => r.json()),
  ]);
  return { mod, meta, std };
}

self.onmessage = async (e) => {
  const { id, source } = e.data;
  try {
    ready = ready || init();
    const { mod, meta, std } = await ready;
    const t0 = performance.now();
    const vfs = new Vfs();
    for (const [p, text] of Object.entries(std)) vfs.write('/work/' + p, text);
    vfs.write('/work/main.mw', source);
    const chunks = [];
    const proc = new Process({
      vfs, cwd: '/work',
      args: ['marrow', '--emit-il', '--nostdlib', '--color', 'never', '--target', 'x86_64-linux', 'main.mw', 'main.ssa'],
      env: ['MARROW_PROGRESS=never'],
      stdout: b => chunks.push(b), stderr: b => chunks.push(b),
    });
    let code;
    try { code = await proc.run(mod, meta.symbols, meta.layout); }
    catch (err) { code = -1; chunks.push(new TextEncoder().encode('internal compiler error: ' + err.message + '\n')); }
    const dec = new TextDecoder();
    const diagnostics = chunks.map(c => dec.decode(c)).join('');
    const il = code === 0 ? dec.decode(vfs.files.get('/work/main.ssa')) : null;
    self.postMessage({ id, code, diagnostics, il, ms: performance.now() - t0 });
  } catch (err) {
    self.postMessage({ id, code: -1, diagnostics: 'could not load the compiler: ' + err.message + '\n', il: null, ms: 0 });
  }
};

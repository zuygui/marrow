// Translates the IL of a program to WebAssembly and runs it. Output is streamed back as it
// is written. The page terminates this worker to stop a program.
import { translate } from './il2wasm.mjs';
import { Process } from './runtime.mjs';

self.onmessage = async (e) => {
  const { il, stdin } = e.data;
  const post = (m) => self.postMessage(m);
  try {
    const t0 = performance.now();
    const t = translate(il);
    post({ type: 'translated', ms: performance.now() - t0, bytes: t.wasm.length });
    const p = new Process({
      args: ['main'],
      stdin: new TextEncoder().encode(stdin || ''),
      stdout: b => post({ type: 'out', data: b }),
      stderr: b => post({ type: 'err', data: b }),
    });
    const t1 = performance.now();
    let code;
    try { code = await p.run(t.wasm, t.symbols, t.layout); }
    catch (err) {
      let msg = err && err.message ? err.message : String(err);
      if (err instanceof RangeError) msg = 'stack overflow (too much recursion)';
      post({ type: 'trap', message: msg });
      code = null;
    }
    post({ type: 'exit', code, ms: performance.now() - t1 });
  } catch (err) {
    post({ type: 'trap', message: err.message });
    post({ type: 'exit', code: null, ms: 0 });
  }
};

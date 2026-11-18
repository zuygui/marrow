// The Marrow playground: the compiler runs in a worker (WebAssembly), the program in another.

const $ = (id) => document.getElementById(id);
const code = $('code'), hl = $('hl'), gutter = $('gutter'), out = $('out'), status = $('status');
const runBtn = $('run'), stopBtn = $('stop'), examplesSel = $('examples');

const RUN_LIMIT_MS = 20000;
const OUTPUT_LIMIT = 2 * 1024 * 1024;

// ---------------------------------------------------------------------------
// highlighting
// ---------------------------------------------------------------------------
const KEYWORDS = new Set(['fn', 'struct', 'enum', 'impl', 'var', 'const', 'ret', 'if', 'else', 'while', 'for', 'break', 'continue', 'match',
  'cast', 'sizeof', 'unsafe', 'raw', 'move', 'true', 'false', 'null', 'self', 'va_start', 'va_arg', 'va_end']);
const PRIMS = new Set(['i8', 'i16', 'i32', 'i64', 'u8', 'u16', 'u32', 'u64', 'f32', 'f64', 'bool', 'rawptr', 'void']);
const TOKEN = /(\/\/[^\n]*)|("(?:[^"\\\n]|\\.)*"?)|('(?:[^'\\\n]|\\.)*'?)|(@[A-Za-z_]\w*)|(\b0[xX][0-9a-fA-F_]+\b|\b\d[\d_]*(?:\.\d+)?\b)|([A-Za-z_]\w*)/g;

function esc(s) { return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;'); }

function highlight(src) {
  let html = '', last = 0, m;
  TOKEN.lastIndex = 0;
  while ((m = TOKEN.exec(src))) {
    html += esc(src.slice(last, m.index));
    last = m.index + m[0].length;
    const t = esc(m[0]);
    if (m[1]) html += `<span class="c">${t}</span>`;
    else if (m[2] || m[3]) html += `<span class="s">${t}</span>`;
    else if (m[4]) html += `<span class="a">${t}</span>`;
    else if (m[5]) html += `<span class="n">${t}</span>`;
    else {
      const w = m[6];
      if (KEYWORDS.has(w)) html += `<span class="k">${t}</span>`;
      else if (PRIMS.has(w) || /^[A-Z]/.test(w)) html += `<span class="t">${t}</span>`;
      else if (src[last] === '(' || (src[last] === ':' && src[last + 1] === ':' && false)) html += `<span class="f">${t}</span>`;
      else html += t;
    }
  }
  html += esc(src.slice(last));
  return html + '\n';
}

function refresh() {
  hl.innerHTML = highlight(code.value);
  const lines = code.value.split('\n').length;
  let g = '';
  for (let i = 1; i <= lines; i++) g += i + '\n';
  gutter.textContent = g;
  syncScroll();
}
function syncScroll() {
  hl.scrollTop = code.scrollTop;
  hl.scrollLeft = code.scrollLeft;
  gutter.scrollTop = code.scrollTop;
}
code.addEventListener('input', () => { refresh(); stateDirty(); });
code.addEventListener('scroll', syncScroll);
window.addEventListener('resize', syncScroll);

code.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') { e.preventDefault(); run(); return; }
  if (e.key === 'Tab') {
    e.preventDefault();
    insert('    ');
  } else if (e.key === 'Enter' && !e.shiftKey && !e.ctrlKey && !e.metaKey && !e.altKey) {
    const v = code.value, pos = code.selectionStart;
    const lineStart = v.lastIndexOf('\n', pos - 1) + 1;
    const indent = /^[ \t]*/.exec(v.slice(lineStart, pos))[0];
    const extra = v.slice(lineStart, pos).trimEnd().endsWith('{') ? '    ' : '';
    e.preventDefault();
    insert('\n' + indent + extra);
  }
});
function insert(text) {
  const s = code.selectionStart, e = code.selectionEnd;
  // execCommand keeps the undo stack; fall back to a plain edit
  if (!document.execCommand || !document.execCommand('insertText', false, text)) {
    code.value = code.value.slice(0, s) + text + code.value.slice(e);
    code.selectionStart = code.selectionEnd = s + text.length;
    refresh();
  }
}

// ---------------------------------------------------------------------------
// output
// ---------------------------------------------------------------------------
let pending = [], scheduled = false, written = 0;
function put(text, cls) {
  pending.push([text, cls]);
  if (!scheduled) { scheduled = true; requestAnimationFrame(flush); }
}
function flush() {
  scheduled = false;
  const stick = out.scrollTop + out.clientHeight >= out.scrollHeight - 30;
  for (const [text, cls] of pending) {
    if (cls) { const s = document.createElement('span'); s.className = cls; s.textContent = text; out.append(s); }
    else out.append(document.createTextNode(text));
  }
  pending = [];
  if (stick) out.scrollTop = out.scrollHeight;
}
function clearOut() { out.textContent = ''; pending = []; written = 0; }
$('clear').onclick = clearOut;

// ---------------------------------------------------------------------------
// workers
// ---------------------------------------------------------------------------
let compiler = null, runner = null, nextId = 1, busy = false, killTimer = null;
function startCompiler() {
  compiler = new Worker('compile-worker.mjs', { type: 'module' });
  compiler.postMessage({ warm: true });
}

function setBusy(b, label) {
  busy = b;
  runBtn.disabled = b;
  stopBtn.disabled = !b;
  if (label) status.textContent = label;
}

function stop(reason) {
  clearTimeout(killTimer);
  if (runner) { runner.terminate(); runner = null; }
  if (compiler && pendingCompile) { compiler.terminate(); startCompiler(); pendingCompile = null; }
  if (reason) put('\n' + reason + '\n', 'e');
  setBusy(false, 'stopped');
}
stopBtn.onclick = () => stop('Stopped.');

let pendingCompile = null;
function compile(source) {
  return new Promise((resolve) => {
    const id = nextId++;
    pendingCompile = id;
    const onmsg = (e) => {
      if (e.data.id !== id) return;
      compiler.removeEventListener('message', onmsg);
      pendingCompile = null;
      resolve(e.data);
    };
    compiler.addEventListener('message', onmsg);
    compiler.postMessage({ id, source });
  });
}

async function run() {
  if (busy) return;
  clearOut();
  setBusy(true, 'compiling…');
  const t0 = performance.now();
  const r = await compile(code.value);
  if (!busy) return; // stopped meanwhile
  const diag = r.diagnostics.trim();
  if (r.code !== 0) {
    put(diag ? diag + '\n' : 'compilation failed\n', 'e');
    setBusy(false, `compilation failed · ${Math.round(r.ms)} ms`);
    return;
  }
  if (diag) put(diag + '\n\n', 'w');
  setBusy(true, 'running…');
  const dec = { out: new TextDecoder(), err: new TextDecoder() };
  let info = '';
  const w = new Worker('run-worker.mjs', { type: 'module' });
  runner = w;
  killTimer = setTimeout(() => { if (runner === w) stop(`Stopped: the program ran for more than ${RUN_LIMIT_MS / 1000} s (an endless loop?).`); }, RUN_LIMIT_MS);
  w.onmessage = (e) => {
    const m = e.data;
    if (m.type === 'translated') info = ` · ${(m.bytes / 1024).toFixed(0)} KB wasm`;
    else if (m.type === 'out' || m.type === 'err') {
      written += m.data.length;
      if (written > OUTPUT_LIMIT) { stop('Stopped: too much output.'); return; }
      put(dec[m.type].decode(m.data, { stream: true }), m.type === 'err' ? 'e' : '');
    } else if (m.type === 'trap') {
      put(`\nruntime error: ${m.message}\n`, 'e');
    } else if (m.type === 'exit') {
      clearTimeout(killTimer);
      runner = null;
      w.terminate();
      if (m.code !== null && m.code !== 0) put(`\nexited with status ${m.code}\n`, 'd');
      setBusy(false, `compiled in ${Math.round(r.ms)} ms${info} · ran in ${Math.round(m.ms)} ms`);
    }
  };
  w.onerror = (e) => { put(`\nruntime error: ${e.message}\n`, 'e'); stop(); };
  w.postMessage({ il: r.il, stdin: $('stdin').value });
}
runBtn.onclick = run;

// ---------------------------------------------------------------------------
// examples, sharing
// ---------------------------------------------------------------------------
let examples = [];
function b64encode(s) {
  const bytes = new TextEncoder().encode(s);
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
}
function b64decode(s) {
  s = s.replace(/-/g, '+').replace(/_/g, '/');
  const bin = atob(s + '='.repeat((4 - (s.length % 4)) % 4));
  return new TextDecoder().decode(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
}
function stateDirty() {
  examplesSel.value = '';
  try { history.replaceState(null, '', location.pathname + location.search); } catch (e) { /* ignore */ }
}
function load(src) { code.value = src; refresh(); }

$('share').onclick = async () => {
  const url = location.origin + location.pathname + '#code=' + b64encode(code.value);
  try { history.replaceState(null, '', url); } catch (e) { /* ignore */ }
  try { await navigator.clipboard.writeText(url); status.textContent = 'link copied'; }
  catch (e) { status.textContent = 'link is in the address bar'; }
};
$('stdin-toggle').onclick = () => $('stdin-box').classList.toggle('open');

examplesSel.onchange = () => {
  const ex = examples.find((x) => x.id === examplesSel.value);
  if (ex) { load(ex.code); try { history.replaceState(null, '', '#example=' + ex.id); } catch (e) { /* ignore */ } }
};

async function init() {
  startCompiler();
  try { examples = await (await fetch('examples.json')).json(); } catch (e) { examples = []; }
  examplesSel.innerHTML = '<option value="">Examples…</option>' + examples.map((x) => `<option value="${x.id}">${x.title}</option>`).join('');
  const h = location.hash;
  let src = null;
  if (h.startsWith('#code=')) { try { src = b64decode(h.slice(6)); } catch (e) { src = null; } }
  else if (h.startsWith('#example=')) {
    const ex = examples.find((x) => x.id === h.slice(9));
    if (ex) { src = ex.code; examplesSel.value = ex.id; }
  }
  load(src !== null ? src : (examples[0] ? examples[0].code : ''));
  if (src === null && examples[0]) examplesSel.value = examples[0].id;
  status.textContent = 'ready · Ctrl/⌘ + Enter to run';
}
init();

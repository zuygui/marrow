// Translates the QBE IL text that the Marrow compiler emits into a WebAssembly module.
//
// The IL uses 64-bit pointers (`l`); the wasm memory is 32-bit, so every address is
// wrapped to i32 when it is used. Locals are the IL temporaries, stack slots (`alloc`)
// live in a shadow stack in linear memory, and the basic blocks run inside a dispatch
// loop (`loop` + `br_table`), which works for any control flow.
//
// translate(ilText, options) -> { wasm: Uint8Array, symbols: {name: address}, layout }
// The only import is `env.syscall(n, a, b, c, d, e, f) -> i64` for `$__marrow_syscall`.

const PAGE = 65536;

// ---------------------------------------------------------------------------
// binary helpers
// ---------------------------------------------------------------------------
class Bytes {
  constructor() { this.a = []; }
  u8(b) { this.a.push(b & 255); return this; }
  bytes(arr) { for (const b of arr) this.a.push(b); return this; }
  u32(v) { // unsigned LEB128
    v = BigInt(v);
    do {
      let b = Number(v & 0x7fn);
      v >>= 7n;
      if (v !== 0n) b |= 0x80;
      this.a.push(b);
    } while (v !== 0n);
    return this;
  }
  s(v) { // signed LEB128
    v = BigInt(v);
    for (;;) {
      const b = Number(v & 0x7fn);
      v >>= 7n;
      if ((v === 0n && !(b & 0x40)) || (v === -1n && (b & 0x40))) { this.a.push(b); break; }
      this.a.push(b | 0x80);
    }
    return this;
  }
  f32(x) { const d = new DataView(new ArrayBuffer(4)); d.setFloat32(0, x, true); for (let i = 0; i < 4; i++) this.a.push(d.getUint8(i)); return this; }
  f64(x) { const d = new DataView(new ArrayBuffer(8)); d.setFloat64(0, x, true); for (let i = 0; i < 8; i++) this.a.push(d.getUint8(i)); return this; }
  name(str) { const e = new TextEncoder().encode(str); this.u32(e.length); this.bytes(e); return this; }
  vec(items, f) { this.u32(items.length); for (const it of items) f(this, it); return this; }
  append(o) { for (const b of o.a) this.a.push(b); return this; }
  get length() { return this.a.length; }
}

function section(id, body) {
  const out = new Bytes();
  out.u8(id).u32(body.length).append(body);
  return out;
}

const VT = { w: 0x7f, l: 0x7e, s: 0x7d, d: 0x7c };

// ---------------------------------------------------------------------------
// parsing
// ---------------------------------------------------------------------------
function parseDataBody(body) {
  // `{ b 10 0, l $x + 8, d d_1.5 }`; items are separated by spaces or commas
  const items = [];
  const toks = [];
  const re = /"(?:[^"\\]|\\.)*"|[^\s,]+/g;
  let m;
  while ((m = re.exec(body))) toks.push(m[0]);
  let i = 0;
  while (i < toks.length) {
    const kind = toks[i++];
    if (kind === 'z') { items.push({ k: 'z', n: Number(toks[i++]) }); continue; }
    // one kind letter followed by one or more values
    for (;;) {
      if (i >= toks.length) break;
      const v = toks[i];
      if (/^[bhwlsdz]$/.test(v)) break;
      i++;
      if (kind === 'b' && v[0] === '"') {
        const bytes = [];
        const s = v.slice(1, -1);
        for (let j = 0; j < s.length; j++) {
          let c = s[j];
          if (c === '\\') {
            c = s[++j];
            const map = { n: 10, t: 9, r: 13, '0': 0, '\\': 92, '"': 34 };
            bytes.push(map[c] !== undefined ? map[c] : c.charCodeAt(0));
          } else bytes.push(c.charCodeAt(0));
        }
        items.push({ k: 'bs', bytes });
      } else if (v[0] === '$') {
        let off = 0;
        if (toks[i] === '+') { off = Number(toks[i + 1]); i += 2; }
        items.push({ k: kind, sym: v.slice(1), off });
      } else items.push({ k: kind, v });
    }
  }
  return items;
}

function parseModule(text) {
  const datas = [];
  const funcs = [];
  let cur = null;
  let blk = null;
  for (let raw of text.split('\n')) {
    const line = raw.trim();
    if (!line || line[0] === '#') continue;
    if (cur) {
      if (line === '}') { funcs.push(cur); cur = null; blk = null; continue; }
      if (line[0] === '@') { blk = { label: line.slice(1), ins: [] }; cur.blocks.push(blk); continue; }
      blk.ins.push(line);
      continue;
    }
    if (/^(export\s+)?data\b/.test(line)) {
      const m = /^(?:export\s+)?data\s+\$(\S+)\s*=\s*(?:align\s+(\d+)\s*)?\{(.*)\}\s*$/.exec(line);
      if (!m) throw new Error('bad data line: ' + line);
      datas.push({ name: m[1], align: m[2] ? Number(m[2]) : 8, items: parseDataBody(m[3]) });
      continue;
    }
    const f = /^(export\s+)?function\s+(?:([wlsd])\s+)?\$([^\s(]+)\s*\((.*)\)\s*\{$/.exec(line);
    if (f) {
      const params = [];
      if (f[4].trim()) {
        for (const p of f[4].split(',')) {
          const pm = /^\s*([wlsd])\s+%(\S+)\s*$/.exec(p);
          if (!pm) throw new Error('bad parameter: ' + p + ' in ' + line);
          params.push({ t: pm[1], name: pm[2] });
        }
      }
      cur = { name: f[3], ret: f[2] || null, exported: !!f[1], params, blocks: [] };
      continue;
    }
    if (line.startsWith('type') || line.startsWith('section')) throw new Error('unsupported: ' + line);
    throw new Error('unexpected line: ' + line);
  }
  return { datas, funcs };
}

// ---------------------------------------------------------------------------
// layout of the static data
// ---------------------------------------------------------------------------
const DATA_BASE = 4096;

function layoutData(datas) {
  const addr = new Map();
  let p = DATA_BASE;
  for (const d of datas) {
    const al = Math.max(1, d.align);
    p = Math.ceil(p / al) * al;
    addr.set(d.name, p);
    d.addr = p;
    let size = 0;
    for (const it of d.items) size += itemSize(it);
    d.size = size;
    p += size;
  }
  return { addr, end: p };
}

function itemSize(it) {
  switch (it.k) {
    case 'b': return 1;
    case 'h': return 2;
    case 'w': case 's': return 4;
    case 'l': case 'd': return 8;
    case 'bs': return it.bytes.length;
    case 'z': return it.n;
  }
  throw new Error('data item ' + it.k);
}

// ---------------------------------------------------------------------------
// function bodies
// ---------------------------------------------------------------------------
const INT_BIN = {
  w: { add: 0x6a, sub: 0x6b, mul: 0x6c, div: 0x6d, udiv: 0x6e, rem: 0x6f, urem: 0x70, and: 0x71, or: 0x72, xor: 0x73, shl: 0x74, sar: 0x75, shr: 0x76 },
  l: { add: 0x7c, sub: 0x7d, mul: 0x7e, div: 0x7f, udiv: 0x80, rem: 0x81, urem: 0x82, and: 0x83, or: 0x84, xor: 0x85, shl: 0x86, sar: 0x87, shr: 0x88 },
};
const FLT_BIN = { s: { add: 0x92, sub: 0x93, mul: 0x94, div: 0x95 }, d: { add: 0xa0, sub: 0xa1, mul: 0xa2, div: 0xa3 } };
const INT_CMP = {
  w: { eq: 0x46, ne: 0x47, slt: 0x48, ult: 0x49, sgt: 0x4a, ugt: 0x4b, sle: 0x4c, ule: 0x4d, sge: 0x4e, uge: 0x4f },
  l: { eq: 0x51, ne: 0x52, slt: 0x53, ult: 0x54, sgt: 0x55, ugt: 0x56, sle: 0x57, ule: 0x58, sge: 0x59, uge: 0x5a },
};
const FLT_CMP = {
  s: { eq: 0x5b, ne: 0x5c, lt: 0x5d, gt: 0x5e, le: 0x5f, ge: 0x60 },
  d: { eq: 0x61, ne: 0x62, lt: 0x63, gt: 0x64, le: 0x65, ge: 0x66 },
};
// opcode, natural alignment exponent
const LOADS = {
  loadw: { w: [0x28, 2], l: [0x34, 2] },
  loadsw: { w: [0x28, 2], l: [0x34, 2] },
  loaduw: { w: [0x28, 2], l: [0x35, 2] },
  loadl: { l: [0x29, 3] },
  loads: { s: [0x2a, 2] },
  loadd: { d: [0x2b, 3] },
  loadsb: { w: [0x2c, 0], l: [0x30, 0] },
  loadub: { w: [0x2d, 0], l: [0x31, 0] },
  loadsh: { w: [0x2e, 1], l: [0x32, 1] },
  loaduh: { w: [0x2f, 1], l: [0x33, 1] },
};
const STORES = {
  storeb: [0x3a, 'w', 0], storeh: [0x3b, 'w', 1], storew: [0x36, 'w', 2],
  storel: [0x37, 'l', 3], stores: [0x38, 's', 2], stored: [0x39, 'd', 3],
};

function splitArgs(s) {
  // split on top-level commas (there are no nested parentheses in IL operands)
  const out = [];
  for (const part of s.split(',')) { const t = part.trim(); if (t) out.push(t); }
  return out;
}

class Ctx {
  constructor(mod) {
    this.mod = mod;
  }
}

function compileFunction(fn, mod, sigOf, funcIndex, typeIndex, tableIndexOf, symAddr, stackGlobal) {
  const local = new Map(); // name -> {idx, t}
  const localTypes = [];   // types of non-param locals
  let nparams = fn.params.length;
  fn.params.forEach((p, i) => local.set(p.name, { idx: i, t: p.t }));
  const fpIdx = nparams;   // frame pointer (i32)
  const pcIdx = nparams + 1;
  localTypes.push('i', 'i'); // i = i32 helper locals (fp, pc)
  function newLocal(name, t) {
    if (local.has(name)) return local.get(name);
    const e = { idx: nparams + localTypes.length, t };
    localTypes.push(t);
    local.set(name, e);
    return e;
  }
  const scratch = {}; // type -> local index for temporaries
  function scratchLocal(t) {
    if (!(t in scratch)) { scratch[t] = nparams + localTypes.length; localTypes.push(t); }
    return scratch[t];
  }

  // pre-pass: result types of all temporaries and frame layout
  const defRe = /^%(\S+)\s*=([wlsd])\s+(\S+)(?:\s+(.*))?$/;
  let frame = 0;
  const allocOff = new Map();
  for (const b of fn.blocks) {
    for (const ins of b.ins) {
      const m = defRe.exec(ins);
      if (!m) continue;
      newLocal(m[1], m[2]);
      const am = /^alloc(\d+)$/.exec(m[3]);
      if (am) {
        const al = Number(am[1]);
        const size = Number((m[4] || '0').trim());
        frame = Math.ceil(frame / al) * al;
        allocOff.set(m[1], frame);
        frame += size;
      }
    }
  }
  frame = Math.ceil(frame / 16) * 16;

  const blockIndex = new Map();
  fn.blocks.forEach((b, i) => blockIndex.set(b.label, i));
  const nb = fn.blocks.length;

  const code = new Bytes();
  const op = (...bs) => code.bytes(bs);

  function constI32(v) { code.u8(0x41).s(BigInt.asIntN(32, BigInt(v))); }
  function constI64(v) { code.u8(0x42).s(BigInt.asIntN(64, BigInt(v))); }

  function operandType(x) {
    if (x[0] === '%') {
      const e = local.get(x.slice(1));
      if (!e) throw new Error(`${fn.name}: unknown temporary ${x}`);
      return e.t;
    }
    return null;
  }

  // push `x` as a value of IL type `t`
  function push(x, t) {
    if (x[0] === '%') {
      const e = local.get(x.slice(1));
      if (!e) throw new Error(`${fn.name}: unknown temporary ${x}`);
      code.u8(0x20).u32(e.idx);
      if (e.t !== t) convertLocal(e.t, t, x);
      return;
    }
    if (x[0] === '$') {
      let name = x.slice(1), off = 0;
      const pm = /^(.*?)\s*\+\s*(\d+)$/.exec(name);
      if (pm) { name = pm[1]; off = Number(pm[2]); }
      let v;
      if (symAddr.has(name)) v = symAddr.get(name) + off;
      else if (tableIndexOf.has(name)) v = tableIndexOf.get(name);
      else throw new Error(`${fn.name}: unknown symbol $${name}`);
      if (t === 'l') constI64(v); else constI32(v);
      return;
    }
    if (x.startsWith('d_')) { code.u8(0x44).f64(parseFloat(x.slice(2))); return; }
    if (x.startsWith('s_')) { code.u8(0x43).f32(parseFloat(x.slice(2))); return; }
    if (/^-?\d+$/.test(x)) {
      if (t === 'l') constI64(x); else if (t === 'w') constI32(x);
      else if (t === 's') code.u8(0x43).f32(Number(x)); else code.u8(0x44).f64(Number(x));
      return;
    }
    throw new Error(`${fn.name}: bad operand ${x}`);
  }
  function convertLocal(from, to, x) {
    if (from === 'l' && to === 'w') code.u8(0xa7); // i32.wrap_i64
    else if (from === 'w' && to === 'l') code.u8(0xad); // i64.extend_i32_u
    else throw new Error(`${fn.name}: ${x} is ${from}, needed ${to}`);
  }
  // push an address (i32) from an `l` operand or a constant
  function pushAddr(x) {
    if (x[0] === '%') {
      const e = local.get(x.slice(1));
      if (!e) throw new Error(`${fn.name}: unknown temporary ${x}`);
      code.u8(0x20).u32(e.idx);
      if (e.t === 'l') code.u8(0xa7);
      return;
    }
    push(x, 'w');
  }
  function setDest(name) {
    const e = local.get(name);
    code.u8(0x21).u32(e.idx);
  }
  function memarg(align, off = 0) { code.u32(align).u32(off); }

  function epilogue() {
    if (frame > 0) {
      code.u8(0x20).u32(fpIdx); constI32(frame); code.u8(0x6a); code.u8(0x24).u32(stackGlobal);
    }
  }
  let curBlock = 0;
  // `br` depth of the loop header from the code of the current block: the blocks that
  // are still open are the ones after it
  const loopDepth = () => nb - 1 - curBlock;
  function jumpTo(label) {
    if (!blockIndex.has(label)) throw new Error(`${fn.name}: unknown block @${label}`);
    constI32(blockIndex.get(label));
    code.u8(0x21).u32(pcIdx);
    code.u8(0x0c).u32(loopDepth());
  }

  // prologue
  if (frame > 0) {
    code.u8(0x23).u32(stackGlobal); constI32(frame); code.u8(0x6b);
    code.u8(0x22).u32(fpIdx); // local.tee fp
    code.u8(0x24).u32(stackGlobal);
    for (const [name, off] of allocOff) {
      code.u8(0x20).u32(fpIdx); constI32(off); code.u8(0x6a); code.u8(0xad); // i64.extend_i32_u
      setDest(name);
    }
  }

  // dispatch: loop { block*nb { local.get pc; br_table } code0; code1; ... }
  code.u8(0x03).u8(0x40);                       // loop
  for (let i = 0; i < nb; i++) code.u8(0x02).u8(0x40); // block x nb
  code.u8(0x20).u32(pcIdx);
  code.u8(0x0e).u32(nb);
  for (let i = 0; i < nb; i++) code.u32(i);
  code.u32(nb - 1);                             // default
  for (let bi = 0; bi < nb; bi++) {
    code.u8(0x0b); // end of block bi: its code follows
    curBlock = bi;
    let terminated = false;
    for (const ins of fn.blocks[bi].ins) {
      terminated = emit(ins) || false;
    }
    if (!terminated) {
      // QBE lets a block fall through to the next one
      if (bi + 1 < nb) {
        constI32(bi + 1);
        code.u8(0x21).u32(pcIdx);
        code.u8(0x0c).u32(loopDepth());
      } else code.u8(0x00);
    }
  }
  code.u8(0x0b); // end loop
  // the loop is exited only through `return`; keep the validator happy
  code.u8(0x00).u8(0x0b); // unreachable; end function

  function emit(ins) {
    let m = defRe.exec(ins);
    if (m) { emitDef(m[1], m[2], m[3], m[4] ? m[4].trim() : ''); return false; }
    const sp = ins.indexOf(' ');
    const opn = sp < 0 ? ins : ins.slice(0, sp);
    const rest = sp < 0 ? '' : ins.slice(sp + 1).trim();
    if (opn === 'jmp') { jumpTo(rest.slice(1)); return true; }
    if (opn === 'jnz') {
      const [c, a, b] = splitArgs(rest);
      constI32(blockIndex.get(a.slice(1)));
      constI32(blockIndex.get(b.slice(1)));
      const ct = operandType(c);
      push(c, ct === 'l' ? 'l' : 'w');
      if (ct === 'l') { code.u8(0x50); code.u8(0x45); } // i64.eqz; i32.eqz  => nonzero
      code.u8(0x1b); // select
      code.u8(0x21).u32(pcIdx);
      code.u8(0x0c).u32(loopDepth());
      return true;
    }
    if (opn === 'ret') {
      if (rest) push(rest, fn.ret);
      epilogue();
      code.u8(0x0f);
      return true;
    }
    if (opn === 'hlt') { code.u8(0x00); return true; }
    if (opn === 'call') { emitCall(null, null, rest); return false; }
    if (opn === 'blit') {
      const [src, dst, n] = splitArgs(rest);
      pushAddr(dst); pushAddr(src); push(n, 'w');
      code.u8(0xfc).u32(10).u8(0).u8(0); // memory.copy
      return false;
    }
    if (STORES[opn]) {
      const [val, addr] = splitArgs(rest);
      const [opc, vt, al] = STORES[opn];
      pushAddr(addr);
      const vty = operandType(val);
      if (vty === 'l' && vt === 'w') { push(val, 'l'); code.u8(0xa7); } else push(val, vt);
      code.u8(opc); memarg(al);
      return false;
    }
    if (opn === 'nop') return false;
    throw new Error(`${fn.name}: unsupported instruction: ${ins}`);
  }

  function emitCall(dest, dt, rest) {
    const cm = /^(\$[^\s(]+|%[^\s(]+)\s*\((.*)\)$/.exec(rest);
    if (!cm) throw new Error(`${fn.name}: bad call: ${rest}`);
    const callee = cm[1];
    const args = [];
    for (const a of splitArgs(cm[2])) {
      if (a === '...') throw new Error(`${fn.name}: variadic call`);
      if (a.startsWith('env ')) continue;
      const am = /^([wlsd])\s+(\S+)$/.exec(a);
      if (!am) throw new Error(`${fn.name}: bad call argument: ${a}`);
      args.push({ t: am[1], x: am[2] });
    }
    for (const a of args) push(a.x, a.t);
    if (callee[0] === '$') {
      const name = callee.slice(1);
      const idx = funcIndex.get(name);
      if (idx === undefined) throw new Error(`${fn.name}: call to undefined $${name}`);
      code.u8(0x10).u32(idx);
      const sig = sigOf(name, args.map(a => a.t), dt);
      if (sig.ret && !dest) code.u8(0x1a); // drop
      else if (!sig.ret && dest) throw new Error(`${fn.name}: $${name} returns nothing`);
    } else {
      push(callee, 'l'); code.u8(0xa7); // table index
      const ti = typeIndex(args.map(a => a.t), dt);
      code.u8(0x11).u32(ti).u8(0);
      if (dt === null) { /* result unknown: assume none */ }
    }
    if (dest) setDest(dest);
  }

  function emitDef(name, t, opn, rest) {
    if (/^alloc\d+$/.test(opn)) return; // set in the prologue
    if (opn === 'call') { emitCall(name, t, rest); return; }
    if (opn === 'copy') {
      const [a] = splitArgs(rest);
      push(a, t); setDest(name); return;
    }
    if (LOADS[opn]) {
      const [addr] = splitArgs(rest);
      const spec = LOADS[opn][t];
      if (!spec) throw new Error(`${fn.name}: ${opn} into ${t}`);
      pushAddr(addr);
      code.u8(spec[0]); memarg(spec[1]);
      setDest(name); return;
    }
    if (opn === 'neg') {
      const [a] = splitArgs(rest);
      if (t === 'w') { constI32(0); push(a, 'w'); code.u8(0x6b); }
      else if (t === 'l') { constI64(0); push(a, 'l'); code.u8(0x7d); }
      else { push(a, t); code.u8(t === 's' ? 0x8c : 0x9a); }
      setDest(name); return;
    }
    if (INT_BIN[t] && INT_BIN[t][opn] !== undefined && (t === 'w' || t === 'l')) {
      const [a, b] = splitArgs(rest);
      push(a, t);
      if (t === 'l' && (opn === 'shl' || opn === 'shr' || opn === 'sar')) {
        // the shift count is a `w` in the IL
        push(b, 'l');
      } else push(b, t);
      code.u8(INT_BIN[t][opn]);
      setDest(name); return;
    }
    if (FLT_BIN[t] && FLT_BIN[t][opn] !== undefined) {
      const [a, b] = splitArgs(rest);
      push(a, t); push(b, t); code.u8(FLT_BIN[t][opn]); setDest(name); return;
    }
    let cm = /^c(eq|ne|sle|slt|sge|sgt|ule|ult|uge|ugt|lt|le|gt|ge)([wlsd])$/.exec(opn);
    if (cm) {
      const [a, b] = splitArgs(rest);
      const ot = cm[2];
      push(a, ot); push(b, ot);
      const table = (ot === 'w' || ot === 'l') ? INT_CMP[ot] : FLT_CMP[ot];
      const opc = table[cm[1]];
      if (opc === undefined) throw new Error(`${fn.name}: ${opn}`);
      code.u8(opc);
      if (t === 'l') code.u8(0xad);
      setDest(name); return;
    }
    // conversions
    const [a] = splitArgs(rest);
    switch (opn) {
      case 'extsw': push(a, 'w'); code.u8(0xac); break;
      case 'extuw': push(a, 'w'); code.u8(0xad); break;
      case 'extsb': if (t === 'l') { push(a, 'w'); code.u8(0xac); code.u8(0xc2); } else { push(a, 'w'); code.u8(0xc0); } break;
      case 'extsh': if (t === 'l') { push(a, 'w'); code.u8(0xac); code.u8(0xc3); } else { push(a, 'w'); code.u8(0xc1); } break;
      case 'extub': push(a, 'w'); if (t === 'l') { code.u8(0xad); constI64(255); code.u8(0x83); } else { constI32(255); code.u8(0x71); } break;
      case 'extuh': push(a, 'w'); if (t === 'l') { code.u8(0xad); constI64(65535); code.u8(0x83); } else { constI32(65535); code.u8(0x71); } break;
      case 'truncd': push(a, 'd'); code.u8(0xb6); break;
      case 'exts': push(a, 's'); code.u8(0xbb); break;
      case 'stosi': push(a, 's'); code.u8(0xfc).u32(t === 'w' ? 0 : 4); break;
      case 'stoui': push(a, 's'); code.u8(0xfc).u32(t === 'w' ? 1 : 5); break;
      case 'dtosi': push(a, 'd'); code.u8(0xfc).u32(t === 'w' ? 2 : 6); break;
      case 'dtoui': push(a, 'd'); code.u8(0xfc).u32(t === 'w' ? 3 : 7); break;
      case 'swtof': push(a, 'w'); code.u8(t === 's' ? 0xb2 : 0xb7); break;
      case 'uwtof': push(a, 'w'); code.u8(t === 's' ? 0xb3 : 0xb8); break;
      case 'sltof': push(a, 'l'); code.u8(t === 's' ? 0xb4 : 0xb9); break;
      case 'ultof': push(a, 'l'); code.u8(t === 's' ? 0xb5 : 0xba); break;
      case 'cast': {
        const st = operandType(a) || (t === 'w' ? 's' : t === 'l' ? 'd' : t === 's' ? 'w' : 'l');
        push(a, st);
        code.u8({ s: 0xbc, d: 0xbd, w: 0xbe, l: 0xbf }[st]);
        break;
      }
      default: throw new Error(`${fn.name}: unsupported operation ${opn} =${t}`);
    }
    setDest(name);
  }

  return { code, localTypes, nparams };
}

// ---------------------------------------------------------------------------
// module
// ---------------------------------------------------------------------------
export function translate(text, opts = {}) {
  const stackSize = opts.stackSize || 8 * 1024 * 1024;
  const mod = parseModule(text);
  const lay = layoutData(mod.datas);
  const symAddr = lay.addr;

  const defined = mod.funcs;
  const definedNames = new Set(defined.map(f => f.name));

  // function signatures; undefined callees become imports
  const sigs = new Map(); // name -> {params:[t], ret}
  for (const f of defined) sigs.set(f.name, { params: f.params.map(p => p.t), ret: f.ret });
  sigs.set('__marrow_syscall', { params: ['l', 'l', 'l', 'l', 'l', 'l', 'l'], ret: 'l' });
  const importNames = [];
  const callRe = /call\s+\$([^\s(]+)\s*\(([^)]*)\)/;
  for (const f of defined) {
    for (const b of f.blocks) for (const ins of b.ins) {
      const m = callRe.exec(ins);
      if (!m) continue;
      if (!definedNames.has(m[1]) && !importNames.includes(m[1])) {
        importNames.push(m[1]);
        if (!sigs.has(m[1])) {
          const dm = /^%\S+\s*=([wlsd])\s+call/.exec(ins);
          const params = m[2].split(',').map(s => s.trim()).filter(Boolean).map(s => s[0]);
          sigs.set(m[1], { params, ret: dm ? dm[1] : null });
        }
      }
    }
  }
  const funcIndex = new Map();
  importNames.forEach((n, i) => funcIndex.set(n, i));
  defined.forEach((f, i) => funcIndex.set(f.name, importNames.length + i));
  // table: index 0 is null, then every defined function
  const tableIndexOf = new Map();
  defined.forEach((f, i) => tableIndexOf.set(f.name, i + 1));

  // types
  const types = [];
  const typeKey = new Map();
  function typeIndexOfSig(params, ret) {
    const key = params.join('') + ':' + (ret || '');
    if (!typeKey.has(key)) { typeKey.set(key, types.length); types.push({ params, ret }); }
    return typeKey.get(key);
  }
  const sigOf = (name) => sigs.get(name);
  const typeIndex = (params, ret) => typeIndexOfSig(params, ret);

  // memory layout: [data][stack][heap...]
  const stackBase = Math.ceil(lay.end / 16) * 16;
  const stackTop = stackBase + stackSize;
  const heapStart = Math.ceil(stackTop / PAGE) * PAGE;
  const stackGlobal = 0;

  // function bodies (also fills the type table)
  for (const n of importNames) typeIndexOfSig(sigs.get(n).params, sigs.get(n).ret);
  for (const f of defined) typeIndexOfSig(f.params.map(p => p.t), f.ret);
  const bodies = defined.map(f =>
    compileFunction(f, mod, sigOf, funcIndex, typeIndex, tableIndexOf, symAddr, stackGlobal));

  const out = new Bytes();
  out.bytes([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);

  const valtype = t => VT[t];
  // type section
  {
    const s = new Bytes();
    s.vec(types, (b, t) => {
      b.u8(0x60);
      b.vec(t.params, (bb, p) => bb.u8(valtype(p)));
      b.u32(t.ret ? 1 : 0);
      if (t.ret) b.u8(valtype(t.ret));
    });
    out.append(section(1, s));
  }
  // import section: syscall plus any other undefined function
  {
    const s = new Bytes();
    s.vec(importNames, (b, n) => {
      b.name('env').name(n === '__marrow_syscall' ? 'syscall' : n);
      b.u8(0x00).u32(typeIndexOfSig(sigs.get(n).params, sigs.get(n).ret));
    });
    out.append(section(2, s));
  }
  // function section
  {
    const s = new Bytes();
    s.vec(defined, (b, f) => b.u32(typeIndexOfSig(f.params.map(p => p.t), f.ret)));
    out.append(section(3, s));
  }
  // table
  {
    const s = new Bytes();
    s.u32(1).u8(0x70).u8(0x00).u32(defined.length + 1);
    out.append(section(4, s));
  }
  // memory (grown by the host)
  {
    const s = new Bytes();
    const pages = heapStart / PAGE + 1;
    s.u32(1).u8(0x00).u32(pages);
    out.append(section(5, s));
  }
  // globals: the shadow stack pointer
  {
    const s = new Bytes();
    s.u32(1).u8(0x7f).u8(0x01).u8(0x41).s(stackTop).u8(0x0b);
    out.append(section(6, s));
  }
  // exports
  {
    const ex = [];
    ex.push(['memory', 0x02, 0]);
    for (const f of defined) if (f.exported || f.name === 'main') ex.push([f.name, 0x00, funcIndex.get(f.name)]);
    ex.push(['__table', 0x01, 0]);
    ex.push(['__stack_pointer', 0x03, 0]);
    const s = new Bytes();
    s.vec(ex, (b, e) => { b.name(e[0]).u8(e[1]).u32(e[2]); });
    out.append(section(7, s));
  }
  // element: table[1..] = defined functions
  {
    const s = new Bytes();
    s.u32(1).u32(0).u8(0x41).s(1).u8(0x0b);
    s.vec(defined, (b, f) => b.u32(funcIndex.get(f.name)));
    out.append(section(9, s));
  }
  // code
  {
    const s = new Bytes();
    s.vec(bodies, (b, body) => {
      const f = new Bytes();
      // local declarations: run-length encoded
      const decl = [];
      for (const t of body.localTypes) {
        const vt = t === 'i' ? 0x7f : VT[t];
        if (decl.length && decl[decl.length - 1][1] === vt) decl[decl.length - 1][0]++;
        else decl.push([1, vt]);
      }
      f.vec(decl, (bb, d) => { bb.u32(d[0]).u8(d[1]); });
      f.append(body.code);
      b.u32(f.length).append(f);
    });
    out.append(section(10, s));
  }
  // data
  {
    const segs = [];
    for (const d of mod.datas) {
      const bytes = new Uint8Array(d.size);
      const dv = new DataView(bytes.buffer);
      let p = 0;
      let nonzero = false;
      for (const it of d.items) {
        switch (it.k) {
          case 'b': bytes[p] = Number(it.v) & 255; p += 1; if (it.v != 0) nonzero = true; break;
          case 'h': dv.setUint16(p, Number(it.v) & 65535, true); p += 2; if (it.v != 0) nonzero = true; break;
          case 'w': dv.setInt32(p, Number(BigInt.asIntN(32, BigInt(it.v))), true); p += 4; if (it.v != 0) nonzero = true; break;
          case 'l': {
            let v;
            if (it.sym !== undefined) {
              const a = symAddr.has(it.sym) ? symAddr.get(it.sym) : tableIndexOf.get(it.sym);
              if (a === undefined) throw new Error('data refers to unknown symbol $' + it.sym);
              v = BigInt(a + it.off);
            } else v = BigInt.asUintN(64, BigInt(it.v));
            dv.setBigUint64(p, v, true); p += 8; if (v !== 0n) nonzero = true; break;
          }
          case 's': dv.setFloat32(p, parseFloat(String(it.v).replace(/^s_/, '')), true); p += 4; nonzero = true; break;
          case 'd': dv.setFloat64(p, parseFloat(String(it.v).replace(/^d_/, '')), true); p += 8; nonzero = true; break;
          case 'bs': bytes.set(it.bytes, p); p += it.bytes.length; nonzero = true; break;
          case 'z': p += it.n; break;
        }
      }
      if (nonzero && d.size > 0) segs.push({ addr: d.addr, bytes });
    }
    const s = new Bytes();
    s.vec(segs, (b, g) => {
      b.u32(0).u8(0x41).s(g.addr).u8(0x0b).u32(g.bytes.length).bytes(g.bytes);
    });
    out.append(section(11, s));
  }

  const symbols = {};
  for (const [n, a] of symAddr) symbols[n] = a;
  return {
    wasm: Uint8Array.from(out.a),
    symbols,
    layout: { dataBase: DATA_BASE, dataEnd: lay.end, stackBase, stackTop, heapStart },
    imports: importNames,
    funcNames: [...importNames, ...defined.map(f => f.name)],
  };
}

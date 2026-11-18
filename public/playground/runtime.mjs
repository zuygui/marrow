// Runs a module made by il2wasm.mjs: emulates the few Linux system calls that Marrow's
// standard library makes (on x86_64 numbering) over an in-memory file system.

import { translate } from './il2wasm.mjs';

const PAGE = 65536;

export class ExitSignal {
  constructor(code) { this.code = code; }
}

export class Vfs {
  constructor(files = {}) {
    this.files = new Map(); // absolute path -> Uint8Array
    this.dirs = new Set(['/']);
    for (const [p, data] of Object.entries(files)) this.write(p, data);
  }
  norm(p, cwd = '/') {
    if (!p.startsWith('/')) p = cwd.replace(/\/$/, '') + '/' + p;
    const out = [];
    for (const part of p.split('/')) {
      if (!part || part === '.') continue;
      if (part === '..') out.pop(); else out.push(part);
    }
    return '/' + out.join('/');
  }
  write(p, data) {
    if (typeof data === 'string') data = new TextEncoder().encode(data);
    this.files.set(p, data);
    let d = p.slice(0, p.lastIndexOf('/'));
    while (d) { this.dirs.add(d); d = d.slice(0, d.lastIndexOf('/')); }
    this.dirs.add('/');
  }
  isDir(p) { return this.dirs.has(p); }
  list(p) {
    const pre = p === '/' ? '/' : p + '/';
    const names = new Map();
    for (const f of this.files.keys()) if (f.startsWith(pre)) {
      const rest = f.slice(pre.length);
      const i = rest.indexOf('/');
      names.set(i < 0 ? rest : rest.slice(0, i), i < 0 ? 8 : 4);
    }
    for (const d of this.dirs) if (d.startsWith(pre) && d !== p) {
      const rest = d.slice(pre.length);
      const i = rest.indexOf('/');
      names.set(i < 0 ? rest : rest.slice(0, i), 4);
    }
    return names;
  }
}

export class Process {
  constructor({ vfs = new Vfs(), stdout = () => {}, stderr = () => {}, stdin = new Uint8Array(0), args = ['a.out'], env = [], cwd = '/', log = null } = {}) {
    this.vfs = vfs;
    this.out = stdout;
    this.err = stderr;
    this.args = args;
    this.env = env;
    this.cwd = cwd;
    this.log = log;
    this.fds = new Map();
    this.fds.set(0, { kind: 'in', data: stdin, pos: 0 });
    this.fds.set(1, { kind: 'out' });
    this.fds.set(2, { kind: 'err' });
    this.nextFd = 3;
    this.dec = new TextDecoder();
  }

  mem8() { return new Uint8Array(this.memory.buffer); }
  view() { return new DataView(this.memory.buffer); }
  cstr(p) {
    const m = this.mem8();
    let e = p;
    while (m[e] !== 0) e++;
    return this.dec.decode(m.subarray(p, e));
  }
  grow(toBytes) {
    const have = this.memory.buffer.byteLength;
    if (toBytes > have) this.memory.grow(Math.ceil((toBytes - have) / PAGE));
  }
  alloc(len) {
    const a = this.heapTop;
    this.heapTop += Math.ceil(len / 16) * 16;
    this.grow(this.heapTop);
    return a;
  }

  syscall = (n, a, b, c, d, e, f) => {
    n = Number(n);
    const r = this.sys(n, a, b, c, d, e, f);
    return BigInt.asIntN(64, BigInt(r));
  };

  sys(n, a, b, c, d, e, f) {
    const A = Number(a), B = Number(b), C = Number(c);
    switch (n) {
      case 0: { // read
        const fd = this.fds.get(A);
        if (!fd) return -9;
        if (fd.kind === 'rand') {
          const m = this.mem8();
          for (let i = 0; i < C; i++) m[B + i] = Math.floor(Math.random() * 256);
          return C;
        }
        if (fd.kind === 'null') return 0;
        if (fd.kind === 'in' || fd.kind === 'file') {
          const n2 = Math.max(0, Math.min(C, fd.data.length - fd.pos));
          this.mem8().set(fd.data.subarray(fd.pos, fd.pos + n2), B);
          fd.pos += n2;
          return n2;
        }
        return -9;
      }
      case 1: { // write
        const fd = this.fds.get(A);
        if (!fd) return -9;
        const chunk = this.mem8().slice(B, B + C);
        if (fd.kind === 'out') this.out(chunk);
        else if (fd.kind === 'err') this.err(chunk);
        else if (fd.kind === 'null') return C;
        else if (fd.kind === 'file' && fd.writable) {
          const need = fd.append ? fd.data.length + C : Math.max(fd.data.length, fd.pos + C);
          if (fd.append) fd.pos = fd.data.length;
          if (need > fd.data.length) { const nd = new Uint8Array(need); nd.set(fd.data); fd.data = nd; }
          fd.data.set(chunk, fd.pos);
          fd.pos += C;
          this.vfs.files.set(fd.path, fd.data);
        } else return -9;
        return C;
      }
      case 2: case 257: { // open, openat
        const pathPtr = n === 2 ? A : B;
        const flags = n === 2 ? B : C;
        const p = this.vfs.norm(this.cstr(pathPtr), this.cwd);
        if (p === '/dev/urandom' || p === '/dev/random' || p === '/dev/null') {
          const fd = this.nextFd++;
          this.fds.set(fd, { kind: p === '/dev/null' ? 'null' : 'rand' });
          return fd;
        }
        const acc = flags & 3;
        const creat = (flags & 64) !== 0, trunc = (flags & 512) !== 0, app = (flags & 1024) !== 0;
        if (this.vfs.isDir(p) && !this.vfs.files.has(p)) {
          const fd = this.nextFd++;
          this.fds.set(fd, { kind: 'dir', path: p, pos: 0, entries: null });
          return fd;
        }
        let data = this.vfs.files.get(p);
        if (!data) {
          if (!creat) return -2;
          data = new Uint8Array(0);
          this.vfs.write(p, data);
        } else if (trunc && acc !== 0) { data = new Uint8Array(0); this.vfs.files.set(p, data); }
        const fd = this.nextFd++;
        this.fds.set(fd, { kind: 'file', path: p, data, pos: 0, writable: acc !== 0, append: app });
        return fd;
      }
      case 3: this.fds.delete(A); return 0; // close
      case 5: case 4: { // fstat, stat
        let size = 0, mode = 0o100644;
        if (n === 5) {
          const fd = this.fds.get(A);
          if (!fd) return -9;
          if (fd.kind === 'file') size = fd.data.length;
          else if (fd.kind === 'dir') mode = 0o040755;
          else mode = 0o020620;
        } else {
          const p = this.vfs.norm(this.cstr(A), this.cwd);
          if (this.vfs.files.has(p)) size = this.vfs.files.get(p).length;
          else if (this.vfs.isDir(p)) mode = 0o040755;
          else return -2;
        }
        const buf = n === 5 ? B : B;
        this.mem8().fill(0, buf, buf + 144);
        const v = this.view();
        v.setUint32(buf + 24, mode, true);
        v.setBigInt64(buf + 48, BigInt(size), true);
        return 0;
      }
      case 8: { // lseek
        const fd = this.fds.get(A);
        if (!fd || fd.kind !== 'file') return -29;
        const off = Number(BigInt.asIntN(64, b));
        const base = C === 0 ? 0 : C === 1 ? fd.pos : fd.data.length;
        fd.pos = base + off;
        return fd.pos;
      }
      case 9: { // mmap (anonymous only)
        const len = Number(b);
        const flags = Number(d);
        if (!(flags & 0x20)) return -22;
        return this.alloc(Math.max(len, 16));
      }
      case 11: return 0; // munmap
      case 10: return 0; // mprotect
      case 12: return 0; // brk
      case 39: return 1; // getpid
      case 60: case 231: throw new ExitSignal(Number(BigInt.asIntN(32, a)));
      case 79: { // getcwd
        const s = new TextEncoder().encode(this.cwd + '\0');
        if (s.length > B) return -34;
        this.mem8().set(s, A);
        return s.length;
      }
      case 80: this.cwd = this.vfs.norm(this.cstr(A), this.cwd); return 0; // chdir
      case 83: { const p = this.vfs.norm(this.cstr(A), this.cwd); this.vfs.dirs.add(p); return 0; } // mkdir
      case 87: { const p = this.vfs.norm(this.cstr(A), this.cwd); return this.vfs.files.delete(p) ? 0 : -2; } // unlink
      case 217: { // getdents64
        const fd = this.fds.get(A);
        if (!fd || fd.kind !== 'dir') return -9;
        if (!fd.entries) fd.entries = [...this.vfs.list(fd.path)];
        let off = 0;
        const v = this.view();
        while (fd.pos < fd.entries.length) {
          const [name, type] = fd.entries[fd.pos];
          const nb = new TextEncoder().encode(name + '\0');
          const reclen = Math.ceil((19 + nb.length) / 8) * 8;
          if (off + reclen > C) break;
          const base = B + off;
          this.mem8().fill(0, base, base + reclen);
          v.setBigUint64(base, BigInt(fd.pos + 1), true);
          v.setBigInt64(base + 8, BigInt(fd.pos + 1), true);
          v.setUint16(base + 16, reclen, true);
          this.mem8()[base + 18] = type;
          this.mem8().set(nb, base + 19);
          off += reclen;
          fd.pos++;
        }
        return off;
      }
      case 228: { // clock_gettime
        const t = performance.now() + (performance.timeOrigin || 0);
        const sec = Math.floor(t / 1000);
        const ns = Math.floor((t - sec * 1000) * 1e6);
        const v = this.view();
        v.setBigInt64(B, BigInt(sec), true);
        v.setBigInt64(B + 8, BigInt(ns), true);
        return 0;
      }
      case 318: { // getrandom
        const m = this.mem8();
        for (let i = 0; i < B; i++) m[A + i] = Math.floor(Math.random() * 256);
        return B;
      }
      case 35: { // nanosleep: busy wait (no shared memory in a plain page)
        const v = this.view();
        const ms = Number(v.getBigInt64(A, true)) * 1000 + Number(v.getBigInt64(A + 8, true)) / 1e6;
        const end = performance.now() + ms;
        while (performance.now() < end) { /* spin */ }
        return 0;
      }
      case 82: { // rename
        const from = this.vfs.norm(this.cstr(A), this.cwd), to = this.vfs.norm(this.cstr(B), this.cwd);
        if (!this.vfs.files.has(from)) return -2;
        this.vfs.write(to, this.vfs.files.get(from));
        this.vfs.files.delete(from);
        return 0;
      }
      case 84: { // rmdir
        const p = this.vfs.norm(this.cstr(A), this.cwd);
        if (!this.vfs.dirs.has(p)) return -2;
        if (this.vfs.list(p).size) return -39;
        this.vfs.dirs.delete(p);
        return 0;
      }
      default:
        if (this.log) this.log(`unsupported syscall ${n}`);
        return -38; // ENOSYS
    }
  }

  // instantiate and run `main(argc, argv)`
  async run(wasm, symbols, layout) {
    const mod = wasm instanceof WebAssembly.Module ? wasm : await WebAssembly.compile(wasm);
    const imports = { env: { syscall: this.syscall } };
    for (const imp of WebAssembly.Module.imports(mod)) {
      if (imp.module === 'env' && imp.name !== 'syscall' && !(imp.name in imports.env)) {
        imports.env[imp.name] = () => { throw new Error('call to undefined function ' + imp.name); };
      }
    }
    const inst = await WebAssembly.instantiate(mod, imports);
    this.instance = inst;
    this.memory = inst.exports.memory;
    this.heapTop = layout.heapStart;

    // argv and envp
    const enc = new TextEncoder();
    const put = (s) => { const b = enc.encode(s + '\0'); const p = this.alloc(b.length); this.mem8().set(b, p); return p; };
    const argp = this.args.map(put);
    const envp = this.env.map(put);
    const vec = this.alloc(8 * (argp.length + envp.length + 2));
    const v = this.view();
    argp.forEach((p, i) => v.setBigUint64(vec + 8 * i, BigInt(p), true));
    v.setBigUint64(vec + 8 * argp.length, 0n, true);
    const envvec = vec + 8 * (argp.length + 1);
    envp.forEach((p, i) => v.setBigUint64(envvec + 8 * i, BigInt(p), true));
    v.setBigUint64(envvec + 8 * envp.length, 0n, true);
    if (symbols.SYS_ENVP !== undefined) v.setBigUint64(symbols.SYS_ENVP, BigInt(envvec), true);

    try {
      const rc = inst.exports.main(this.args.length, BigInt(vec));
      return rc;
    } catch (e) {
      if (e instanceof ExitSignal) return e.code;
      throw e;
    }
  }
}

export async function runIl(il, opts = {}) {
  const t = translate(il, opts);
  const p = new Process(opts.process || {});
  const code = await p.run(t.wasm, t.symbols, t.layout);
  return { code, process: p, translation: t };
}

//! Compilation targets and the tiny assembly runtime used by `--nostdlib`.

use crate::diagnostic::Diagnostic;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Arch {
    X86_64,
    Aarch64,
    Riscv64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Os {
    Linux,
    MacOs,
    Windows,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Target {
    pub arch: Arch,
    pub os: Os,
}

impl Target {
    pub fn host() -> Result<Target, Diagnostic> {
        let arch = if cfg!(target_arch = "x86_64") {
            Arch::X86_64
        } else if cfg!(target_arch = "aarch64") {
            Arch::Aarch64
        } else if cfg!(target_arch = "riscv64") {
            Arch::Riscv64
        } else {
            return Err(Diagnostic::error("unsupported host architecture").help("pass `--target <arch>-<os>` explicitly"));
        };
        let os = if cfg!(target_os = "linux") {
            Os::Linux
        } else if cfg!(target_os = "macos") {
            Os::MacOs
        } else if cfg!(target_os = "windows") {
            Os::Windows
        } else {
            return Err(Diagnostic::error("unsupported host operating system").help("pass `--target <arch>-<os>` explicitly"));
        };
        Ok(Target { arch, os })
    }

    /// Parses `<arch>-<os>` (`x86_64-linux`, `aarch64-linux`, `riscv64-linux`, ...).
    pub fn parse(s: &str) -> Result<Target, Diagnostic> {
        let bad = || {
            Diagnostic::error(format!("unknown target `{}`", s))
                .help("expected `<arch>-<os>` with arch in x86_64, aarch64, riscv64 and os in linux, macos, windows")
        };
        let (a, o) = s.split_once('-').ok_or_else(bad)?;
        let arch = match a {
            "x86_64" | "amd64" => Arch::X86_64,
            "aarch64" | "arm64" => Arch::Aarch64,
            "riscv64" | "rv64" => Arch::Riscv64,
            _ => return Err(bad()),
        };
        let os = match o {
            "linux" => Os::Linux,
            "macos" | "darwin" => Os::MacOs,
            "windows" => Os::Windows,
            _ => return Err(bad()),
        };
        Ok(Target { arch, os })
    }

    pub fn is_host(&self) -> bool {
        Target::host().map(|h| h == *self).unwrap_or(false)
    }

    /// Name used to select the platform tables, e.g. `linux_x86_64`
    /// (`@import("sys/{target}")` expands to `sys/linux_x86_64`).
    pub fn dir_name(&self) -> String {
        let os = match self.os {
            Os::Linux => "linux",
            Os::MacOs => "macos",
            Os::Windows => "windows",
        };
        let arch = match self.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
            Arch::Riscv64 => "riscv64",
        };
        format!("{}_{}", os, arch)
    }

    /// Value of QBE's `-t` option.
    pub fn qbe_target(&self) -> &'static str {
        match (self.arch, self.os) {
            (Arch::X86_64, Os::MacOs) => "amd64_apple",
            (Arch::X86_64, _) => "amd64_sysv",
            (Arch::Aarch64, Os::MacOs) => "arm64_apple",
            (Arch::Aarch64, _) => "arm64",
            (Arch::Riscv64, _) => "rv64",
        }
    }

    pub fn check_nostdlib_supported(&self) -> Result<(), Diagnostic> {
        match (self.os, self.arch) {
            (Os::Linux, _) | (Os::MacOs, Arch::X86_64 | Arch::Aarch64) | (Os::Windows, Arch::X86_64) => Ok(()),
            _ => Err(Diagnostic::error(format!("`--nostdlib` is not supported on {} yet", self.dir_name()))
                .note("supported targets: Linux (x86_64, aarch64, riscv64), macOS (x86_64, aarch64), Windows (x86_64)")),
        }
    }

    /// On Linux the program is fully freestanding (`_start` is ours). On macOS
    /// and Windows the OS toolchain still provides the process entry point and
    /// links its system library; Marrow code just never calls into the C
    /// library, it talks to the kernel (macOS) or to kernel32 (Windows).
    pub fn freestanding_link(&self) -> bool {
        self.os == Os::Linux
    }

    /// Assembly appended to the generated code. The `__marrow_syscall` stub
    /// is always present (std/platform binds it); the process entry point is
    /// only added for freestanding Linux builds (`--nostdlib`).
    /// QBE emits ELF flavoured assembly for its System V target; COFF
    /// assemblers (Windows) reject the ELF-only `.type`/`.size`/note directives.
    pub fn adapt_asm(&self, asm: &str) -> String {
        if self.os != Os::Windows {
            return asm.to_string();
        }
        let mut out = String::with_capacity(asm.len());
        for line in asm.lines() {
            let t = line.trim_start();
            if t.starts_with(".type ") || t.starts_with(".size ") || t.starts_with(".section .note.GNU-stack") {
                continue;
            }
            out.push_str(line);
            out.push('\n');
        }
        out
    }

    pub fn runtime_asm(&self, nostdlib: bool) -> String {
        let (entry, stub) = match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) => (X86_64_LINUX, X86_64_LINUX_SYSCALL),
            (Os::Linux, Arch::Aarch64) => (AARCH64_LINUX, AARCH64_LINUX_SYSCALL),
            (Os::Linux, Arch::Riscv64) => (RISCV64_LINUX, RISCV64_LINUX_SYSCALL),
            (Os::MacOs, Arch::X86_64) => ("", X86_64_MACOS),
            (Os::MacOs, _) => ("", AARCH64_MACOS),
            (Os::Windows, _) => ("", X86_64_WINDOWS),
        };
        let ctx = match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) | (Os::Windows, _) => X86_64_CTX,
            (Os::Linux, Arch::Aarch64) => AARCH64_CTX,
            (Os::Linux, Arch::Riscv64) => RISCV64_CTX,
            (Os::MacOs, Arch::X86_64) => X86_64_MACOS_CTX,
            (Os::MacOs, _) => AARCH64_MACOS_CTX,
        };
        let mut out = String::new();
        if nostdlib {
            out.push_str(entry);
        }
        out.push_str(stub);
        out.push_str(ctx);
        out
    }
}

const X86_64_LINUX: &str = "
.text
.globl _start
_start:
\txorl %ebp, %ebp
\tmovq (%rsp), %rdi
\tleaq 8(%rsp), %rsi
\tandq $-16, %rsp
\tcall main
\tmovl %eax, %edi
\tmovl $60, %eax
\tsyscall
";

const X86_64_LINUX_SYSCALL: &str = "
.text
.globl __marrow_syscall
__marrow_syscall:
\tmovq %rdi, %rax
\tmovq %rsi, %rdi
\tmovq %rdx, %rsi
\tmovq %rcx, %rdx
\tmovq %r8, %r10
\tmovq %r9, %r8
\tmovq 8(%rsp), %r9
\tsyscall
\tret
.section .note.GNU-stack,\"\",@progbits
";

const AARCH64_LINUX: &str = "
.text
.globl _start
_start:
\tmov x29, #0
\tldr x0, [sp]
\tadd x1, sp, #8
\tbl main
\tmov x8, #93
\tsvc #0
";

const AARCH64_LINUX_SYSCALL: &str = "
.text
.globl __marrow_syscall
__marrow_syscall:
\tmov x8, x0
\tmov x0, x1
\tmov x1, x2
\tmov x2, x3
\tmov x3, x4
\tmov x4, x5
\tmov x5, x6
\tsvc #0
\tret
.section .note.GNU-stack,\"\",@progbits
";

const RISCV64_LINUX: &str = "
.text
.globl _start
_start:
\tmv fp, zero
\tld a0, 0(sp)
\taddi a1, sp, 8
\tcall main
\tli a7, 93
\tecall
";

const RISCV64_LINUX_SYSCALL: &str = "
.text
.globl __marrow_syscall
__marrow_syscall:
\tmv a7, a0
\tmv a0, a1
\tmv a1, a2
\tmv a2, a3
\tmv a3, a4
\tmv a4, a5
\tmv a5, a6
\tecall
\tret
.section .note.GNU-stack,\"\",@progbits
";

// macOS: BSD syscalls. The kernel reports errors with the carry flag and a
// positive errno; the stub turns that into the Linux convention (-errno).
// Apple does not guarantee the numbers across releases (see std/platform).
const X86_64_MACOS: &str = "
.text
.globl ___marrow_syscall
___marrow_syscall:
\tmovq %rdi, %rax
\taddq $0x2000000, %rax
\tmovq %rsi, %rdi
\tmovq %rdx, %rsi
\tmovq %rcx, %rdx
\tmovq %r8, %r10
\tmovq %r9, %r8
\tmovq 8(%rsp), %r9
\tsyscall
\tjnc 1f
\tnegq %rax
1:
\tret
";

const AARCH64_MACOS: &str = "
.text
.globl ___marrow_syscall
.p2align 2
___marrow_syscall:
\tmov x16, x0
\tmov x0, x1
\tmov x1, x2
\tmov x2, x3
\tmov x3, x4
\tmov x4, x5
\tmov x5, x6
\tsvc #0x80
\tb.cc 1f
\tneg x0, x0
1:
\tret
";

// Windows has no stable system call interface, so `n` is an index into a
// table of kernel32 functions and the stub converts from the System V
// convention (used by QBE-generated code) to the Win64 one.
const X86_64_WINDOWS: &str = "
.text
.globl __marrow_syscall
__marrow_syscall:
\tpushq %rbp
\tmovq %rsp, %rbp
\tsubq $64, %rsp
\tmovq 16(%rbp), %rax
\tmovq %rax, 40(%rsp)
\tmovq %r9, 32(%rsp)
\tleaq __marrow_k32_table(%rip), %rax
\tmovq (%rax,%rdi,8), %rax
\tmovq (%rax), %r10
\tmovq %r8, %r11
\tmovq %rcx, %r8
\tmovq %r11, %r9
\tmovq %rsi, %rcx
\tcall *%r10
\tleave
\tret
.section .rdata,\"dr\"
.p2align 3
__marrow_k32_table:
\t.quad __imp_ExitProcess
\t.quad __imp_GetStdHandle
\t.quad __imp_WriteFile
\t.quad __imp_ReadFile
\t.quad __imp_CloseHandle
\t.quad __imp_VirtualAlloc
\t.quad __imp_VirtualFree
\t.quad __imp_CreateFileA
";

// Context switch used by std/task.mw coroutines:
//     __marrow_ctx_switch(from: *i64, to: i64)
// pushes the callee-saved registers on the current stack, stores the stack
// pointer in *from, installs the stack pointer `to` and pops the registers
// that were saved there; the final `ret` resumes the other context.  A new
// context is a fabricated frame whose return slot holds the entry function.
const X86_64_CTX: &str = "
.text
.globl __marrow_ctx_switch
__marrow_ctx_switch:
\tpushq %rbp
\tpushq %rbx
\tpushq %r12
\tpushq %r13
\tpushq %r14
\tpushq %r15
\tmovq %rsp, (%rdi)
\tmovq %rsi, %rsp
\tpopq %r15
\tpopq %r14
\tpopq %r13
\tpopq %r12
\tpopq %rbx
\tpopq %rbp
\tret
";

const X86_64_MACOS_CTX: &str = "
.text
.globl ___marrow_ctx_switch
___marrow_ctx_switch:
\tpushq %rbp
\tpushq %rbx
\tpushq %r12
\tpushq %r13
\tpushq %r14
\tpushq %r15
\tmovq %rsp, (%rdi)
\tmovq %rsi, %rsp
\tpopq %r15
\tpopq %r14
\tpopq %r13
\tpopq %r12
\tpopq %rbx
\tpopq %rbp
\tret
";

const AARCH64_CTX: &str = "
.text
.globl __marrow_ctx_switch
.p2align 2
__marrow_ctx_switch:
\tsub sp, sp, #176
\tstp x19, x20, [sp, #0]
\tstp x21, x22, [sp, #16]
\tstp x23, x24, [sp, #32]
\tstp x25, x26, [sp, #48]
\tstp x27, x28, [sp, #64]
\tstp x29, x30, [sp, #80]
\tstp d8, d9, [sp, #96]
\tstp d10, d11, [sp, #112]
\tstp d12, d13, [sp, #128]
\tstp d14, d15, [sp, #144]
\tmov x2, sp
\tstr x2, [x0]
\tmov sp, x1
\tldp x19, x20, [sp, #0]
\tldp x21, x22, [sp, #16]
\tldp x23, x24, [sp, #32]
\tldp x25, x26, [sp, #48]
\tldp x27, x28, [sp, #64]
\tldp x29, x30, [sp, #80]
\tldp d8, d9, [sp, #96]
\tldp d10, d11, [sp, #112]
\tldp d12, d13, [sp, #128]
\tldp d14, d15, [sp, #144]
\tadd sp, sp, #176
\tret
.section .note.GNU-stack,\"\",@progbits
";

const AARCH64_MACOS_CTX: &str = "
.text
.globl ___marrow_ctx_switch
.p2align 2
___marrow_ctx_switch:
\tsub sp, sp, #176
\tstp x19, x20, [sp, #0]
\tstp x21, x22, [sp, #16]
\tstp x23, x24, [sp, #32]
\tstp x25, x26, [sp, #48]
\tstp x27, x28, [sp, #64]
\tstp x29, x30, [sp, #80]
\tstp d8, d9, [sp, #96]
\tstp d10, d11, [sp, #112]
\tstp d12, d13, [sp, #128]
\tstp d14, d15, [sp, #144]
\tmov x2, sp
\tstr x2, [x0]
\tmov sp, x1
\tldp x19, x20, [sp, #0]
\tldp x21, x22, [sp, #16]
\tldp x23, x24, [sp, #32]
\tldp x25, x26, [sp, #48]
\tldp x27, x28, [sp, #64]
\tldp x29, x30, [sp, #80]
\tldp d8, d9, [sp, #96]
\tldp d10, d11, [sp, #112]
\tldp d12, d13, [sp, #128]
\tldp d14, d15, [sp, #144]
\tadd sp, sp, #176
\tret
";

const RISCV64_CTX: &str = "
.text
.globl __marrow_ctx_switch
__marrow_ctx_switch:
\taddi sp, sp, -208
\tsd ra, 0(sp)
\tsd s0, 8(sp)
\tsd s1, 16(sp)
\tsd s2, 24(sp)
\tsd s3, 32(sp)
\tsd s4, 40(sp)
\tsd s5, 48(sp)
\tsd s6, 56(sp)
\tsd s7, 64(sp)
\tsd s8, 72(sp)
\tsd s9, 80(sp)
\tsd s10, 88(sp)
\tsd s11, 96(sp)
\tfsd fs0, 104(sp)
\tfsd fs1, 112(sp)
\tfsd fs2, 120(sp)
\tfsd fs3, 128(sp)
\tfsd fs4, 136(sp)
\tfsd fs5, 144(sp)
\tfsd fs6, 152(sp)
\tfsd fs7, 160(sp)
\tfsd fs8, 168(sp)
\tfsd fs9, 176(sp)
\tfsd fs10, 184(sp)
\tfsd fs11, 192(sp)
\tsd sp, 0(a0)
\tmv sp, a1
\tld ra, 0(sp)
\tld s0, 8(sp)
\tld s1, 16(sp)
\tld s2, 24(sp)
\tld s3, 32(sp)
\tld s4, 40(sp)
\tld s5, 48(sp)
\tld s6, 56(sp)
\tld s7, 64(sp)
\tld s8, 72(sp)
\tld s9, 80(sp)
\tld s10, 88(sp)
\tld s11, 96(sp)
\tfld fs0, 104(sp)
\tfld fs1, 112(sp)
\tfld fs2, 120(sp)
\tfld fs3, 128(sp)
\tfld fs4, 136(sp)
\tfld fs5, 144(sp)
\tfld fs6, 152(sp)
\tfld fs7, 160(sp)
\tfld fs8, 168(sp)
\tfld fs9, 176(sp)
\tfld fs10, 184(sp)
\tfld fs11, 192(sp)
\taddi sp, sp, 208
\tret
";

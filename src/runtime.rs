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
    pub fn runtime_asm(&self, nostdlib: bool) -> String {
        let (entry, stub) = match (self.os, self.arch) {
            (Os::Linux, Arch::X86_64) => (X86_64_LINUX, X86_64_LINUX_SYSCALL),
            (Os::Linux, Arch::Aarch64) => (AARCH64_LINUX, AARCH64_LINUX_SYSCALL),
            (Os::Linux, Arch::Riscv64) => (RISCV64_LINUX, RISCV64_LINUX_SYSCALL),
            (Os::MacOs, Arch::X86_64) => ("", X86_64_MACOS),
            (Os::MacOs, _) => ("", AARCH64_MACOS),
            (Os::Windows, _) => ("", X86_64_WINDOWS),
        };
        let mut out = String::new();
        if nostdlib {
            out.push_str(entry);
        }
        out.push_str(stub);
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

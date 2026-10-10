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
        if self.os == Os::Linux {
            return Ok(());
        }
        Err(Diagnostic::error(format!("`--nostdlib` is not supported on {} yet", self.dir_name()))
            .note("only Linux has a stable raw system call interface")
            .help("macOS and Windows should go through their system libraries instead"))
    }

    /// Assembly appended to the generated code: the process entry point and
    /// `__marrow_syscall(n, a, b, c, d, e, f)`, which `std/os.mw` binds.
    pub fn runtime_asm(&self) -> &'static str {
        match self.arch {
            Arch::X86_64 => X86_64_LINUX,
            Arch::Aarch64 => AARCH64_LINUX,
            Arch::Riscv64 => RISCV64_LINUX,
        }
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

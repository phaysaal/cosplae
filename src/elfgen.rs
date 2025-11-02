use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::ir::{Instr, ProgramIR};

/// x86-64 machine code compiler that generates native ELF64 executables
pub struct Compiler {
    code: Vec<u8>,
    data: Vec<u8>,
    data_labels: Vec<(usize, String)>, // (offset in data section, label name)
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            data: Vec::new(),
            data_labels: Vec::new(),
        }
    }

    /// Compile the IR program to an ELF binary
    pub fn compile_program<P: AsRef<Path>>(
        &mut self,
        prog: &ProgramIR,
        out_path: P,
    ) -> std::io::Result<()> {
        // Find main function
        let main_idx = prog.main_index().expect("no `main` function found");
        let main_func = &prog.funcs[main_idx];

        // Generate prologue
        self.emit_prologue(main_func.n_locals);

        // Compile main function body
        for instr in &main_func.code {
            self.compile_instr(instr);
        }

        // If we reach here without explicit return, exit with code 0
        // xor edi, edi (exit code 0)
        self.code.extend_from_slice(&[0x31, 0xFF]);
        // mov rax, 60 (sys_exit)
        self.code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00]);
        // syscall
        self.code.extend_from_slice(&[0x0F, 0x05]);

        // Generate ELF binary
        self.generate_elf(out_path)
    }

    /// Emit function prologue: setup stack frame for locals
    fn emit_prologue(&mut self, n_locals: usize) {
        // push rbp
        self.code.push(0x55);

        // mov rbp, rsp
        self.code.extend_from_slice(&[0x48, 0x89, 0xE5]);

        // sub rsp, n_locals * 8 (allocate space for locals)
        if n_locals > 0 {
            let stack_size = (n_locals * 8) as i32;
            if stack_size <= 127 {
                // sub rsp, imm8
                self.code.extend_from_slice(&[0x48, 0x83, 0xEC, stack_size as u8]);
            } else {
                // sub rsp, imm32
                self.code.extend_from_slice(&[0x48, 0x81, 0xEC]);
                self.code.extend_from_slice(&stack_size.to_le_bytes());
            }
        }
    }


    /// Compile a single IR instruction to x86-64 machine code
    fn compile_instr(&mut self, instr: &Instr) {
        match instr {
            Instr::PushI32(n) => self.emit_push_i32(*n),
            Instr::Pop => self.emit_pop_discard(),
            Instr::Load(idx) => self.emit_load(*idx),
            Instr::Store(idx) => self.emit_store(*idx),
            Instr::Add => self.emit_add(),
            Instr::Sub => self.emit_sub(),
            Instr::Mul => self.emit_mul(),
            Instr::Div => self.emit_div(),
            Instr::Print => self.emit_print(),
            Instr::Ret => self.emit_return(),
        }
    }

    // ========== Stack Operations ==========

    /// Push a 32-bit constant onto the stack
    fn emit_push_i32(&mut self, value: i32) {
        // push imm32 (sign-extended to 64-bit)
        self.code.push(0x68);
        self.code.extend_from_slice(&value.to_le_bytes());
    }

    /// Pop and discard top of stack
    fn emit_pop_discard(&mut self) {
        // add rsp, 8 (discard top value)
        self.code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]);
    }

    // ========== Local Variable Access ==========

    /// Load local variable onto stack: push qword [rbp - 8*(idx+1)]
    fn emit_load(&mut self, idx: usize) {
        let offset = ((idx + 1) * 8) as i32;

        if offset <= 128 {
            // push qword [rbp - offset] using 8-bit displacement
            // This is: ff 75 -offset
            self.code.extend_from_slice(&[0xFF, 0x75, (256 - offset) as u8]);
        } else {
            // For larger offsets, need to use mov + push
            // mov rax, [rbp - offset]
            self.code.extend_from_slice(&[0x48, 0x8B, 0x85]);
            self.code.extend_from_slice(&(-offset).to_le_bytes());
            // push rax
            self.code.push(0x50);
        }
    }

    /// Store top of stack to local variable: pop into [rbp - 8*(idx+1)]
    fn emit_store(&mut self, idx: usize) {
        let offset = ((idx + 1) * 8) as i32;

        // pop rax
        self.code.push(0x58);

        if offset <= 128 {
            // mov [rbp - offset], rax using 8-bit displacement
            self.code.extend_from_slice(&[0x48, 0x89, 0x45, (256 - offset) as u8]);
        } else {
            // mov [rbp - offset], rax using 32-bit displacement
            self.code.extend_from_slice(&[0x48, 0x89, 0x85]);
            self.code.extend_from_slice(&(-offset).to_le_bytes());
        }
    }

    // ========== Arithmetic Operations ==========

    /// Add: pop b, pop a, push (a + b)
    fn emit_add(&mut self) {
        // pop rbx
        self.code.push(0x5B);
        // pop rax
        self.code.push(0x58);
        // add rax, rbx
        self.code.extend_from_slice(&[0x48, 0x01, 0xD8]);
        // push rax
        self.code.push(0x50);
    }

    /// Sub: pop b, pop a, push (a - b)
    fn emit_sub(&mut self) {
        // pop rbx
        self.code.push(0x5B);
        // pop rax
        self.code.push(0x58);
        // sub rax, rbx
        self.code.extend_from_slice(&[0x48, 0x29, 0xD8]);
        // push rax
        self.code.push(0x50);
    }

    /// Mul: pop b, pop a, push (a * b)
    fn emit_mul(&mut self) {
        // pop rbx
        self.code.push(0x5B);
        // pop rax
        self.code.push(0x58);
        // imul rax, rbx
        self.code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC3]);
        // push rax
        self.code.push(0x50);
    }

    /// Div: pop b, pop a, push (a / b)
    fn emit_div(&mut self) {
        // pop rbx (divisor)
        self.code.push(0x5B);
        // pop rax (dividend)
        self.code.push(0x58);
        // cqo (sign extend rax to rdx:rax)
        self.code.extend_from_slice(&[0x48, 0x99]);
        // idiv rbx
        self.code.extend_from_slice(&[0x48, 0xF7, 0xFB]);
        // push rax (quotient)
        self.code.push(0x50);
    }

    // ========== I/O Operations ==========

    /// Print: pop value and print to stdout as decimal number
    fn emit_print(&mut self) {
        // TODO: Implement proper integer-to-string conversion
        // For now, this is a no-op - just pop the value and discard it
        // pop rax (value to print)
        self.code.push(0x58);

        // Future implementation will:
        // 1. Convert integer in rax to ASCII decimal string
        // 2. Use sys_write to output to stdout
        // 3. Call syscall
    }

    /// Return from function: exit program with return value
    fn emit_return(&mut self) {
        // pop rdi (return value becomes exit code)
        self.code.push(0x5F);

        // mov rax, 60 (sys_exit)
        self.code.extend_from_slice(&[0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00]);

        // syscall
        self.code.extend_from_slice(&[0x0F, 0x05]);
    }

    // ========== ELF Generation ==========

    /// Generate the final ELF64 executable
    fn generate_elf<P: AsRef<Path>>(&self, out_path: P) -> std::io::Result<()> {
        const BASE_VADDR: u64 = 0x400000;
        const OFF_ELF_HDR: u64 = 0x0000;
        const OFF_PROG_HDR: u64 = 0x0040;
        const OFF_CODE: u64 = 0x1000;

        let code_vaddr = BASE_VADDR + OFF_CODE;

        // Build the complete segment (code + data)
        let mut segment = self.code.clone();
        segment.extend_from_slice(&self.data);

        // ---- ELF header (64 bytes) ----
        let mut elf: Vec<u8> = Vec::new();

        // e_ident
        elf.extend_from_slice(&[
            0x7F, b'E', b'L', b'F',   // EI_MAG
            0x02,                      // EI_CLASS = ELFCLASS64
            0x01,                      // EI_DATA = little-endian
            0x01,                      // EI_VERSION
            0x00,                      // EI_OSABI = System V
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // EI_PAD
        ]);

        // e_type, e_machine, e_version
        elf.extend_from_slice(&u16::to_le_bytes(2));       // ET_EXEC
        elf.extend_from_slice(&u16::to_le_bytes(0x3E));    // EM_X86_64
        elf.extend_from_slice(&u32::to_le_bytes(1));       // EV_CURRENT

        // e_entry (entry point - start of code)
        elf.extend_from_slice(&u64::to_le_bytes(code_vaddr));

        // e_phoff, e_shoff
        elf.extend_from_slice(&u64::to_le_bytes(OFF_PROG_HDR));
        elf.extend_from_slice(&u64::to_le_bytes(0));       // no section headers

        // e_flags
        elf.extend_from_slice(&u32::to_le_bytes(0));

        // e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx
        elf.extend_from_slice(&u16::to_le_bytes(64));      // e_ehsize
        elf.extend_from_slice(&u16::to_le_bytes(56));      // e_phentsize
        elf.extend_from_slice(&u16::to_le_bytes(1));       // e_phnum
        elf.extend_from_slice(&u16::to_le_bytes(0));       // e_shentsize
        elf.extend_from_slice(&u16::to_le_bytes(0));       // e_shnum
        elf.extend_from_slice(&u16::to_le_bytes(0));       // e_shstrndx

        // Pad to program header offset
        while elf.len() < OFF_PROG_HDR as usize {
            elf.push(0);
        }

        // ---- Program header (56 bytes) ----
        elf.extend_from_slice(&u32::to_le_bytes(1));           // PT_LOAD
        elf.extend_from_slice(&u32::to_le_bytes(5));           // PF_R | PF_X
        elf.extend_from_slice(&u64::to_le_bytes(OFF_CODE));    // p_offset
        elf.extend_from_slice(&u64::to_le_bytes(code_vaddr));  // p_vaddr
        elf.extend_from_slice(&u64::to_le_bytes(code_vaddr));  // p_paddr
        elf.extend_from_slice(&u64::to_le_bytes(segment.len() as u64)); // p_filesz
        elf.extend_from_slice(&u64::to_le_bytes(segment.len() as u64)); // p_memsz
        elf.extend_from_slice(&u64::to_le_bytes(0x1000));      // p_align

        // Pad to code offset
        while elf.len() < OFF_CODE as usize {
            elf.push(0);
        }

        // Append code and data
        elf.extend_from_slice(&segment);

        // Write to file with executable permissions
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o755)
            .open(out_path)?;
        f.write_all(&elf)?;
        f.flush()?;

        Ok(())
    }
}

/// Legacy function - kept for backwards compatibility
pub fn emit_min_elf_hello<P: AsRef<Path>>(out_path: P) -> std::io::Result<()> {
    // Original minimal ELF "Hello\n" generator
    const BASE_VADDR: u64 = 0x400000;
    const OFF_ELF_HDR: u64 = 0x0000;
    const OFF_PROG_HDR: u64 = 0x0040;
    const OFF_SEG: u64 = 0x1000;
    const VADDR_SEG: u64 = BASE_VADDR + OFF_SEG;

    let mut code: Vec<u8> = vec![
        0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00,       // mov rax, 1
        0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00,       // mov rdi, 1
        0x48, 0x8D, 0x35, 0, 0, 0, 0,                   // lea rsi, [rip+disp32]
        0x48, 0xC7, 0xC2, 0x06, 0x00, 0x00, 0x00,       // mov rdx, 6
        0x0F, 0x05,                                     // syscall
        0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00,       // mov rax, 60
        0x48, 0x31, 0xFF,                               // xor rdi, rdi
        0x0F, 0x05,                                     // syscall
    ];
    let lea_disp32_offset_in_code = 12 + 3;

    let msg = b"Hello\n";

    let code_start_file_off = OFF_SEG as usize;
    let msg_file_off = code_start_file_off + code.len();
    let lea_next_ip_file_off = code_start_file_off + (12 + 7);
    let disp = (msg_file_off as i64) - (lea_next_ip_file_off as i64);
    let disp_bytes = (disp as i32).to_le_bytes();
    code[lea_disp32_offset_in_code + 0] = disp_bytes[0];
    code[lea_disp32_offset_in_code + 1] = disp_bytes[1];
    code[lea_disp32_offset_in_code + 2] = disp_bytes[2];
    code[lea_disp32_offset_in_code + 3] = disp_bytes[3];

    let mut seg: Vec<u8> = Vec::with_capacity(code.len() + msg.len());
    seg.extend_from_slice(&code);
    seg.extend_from_slice(msg);

    let mut elf: Vec<u8> = Vec::with_capacity(0x1000 + seg.len());
    elf.extend_from_slice(&[
        0x7F, b'E', b'L', b'F',
        0x02, 0x01, 0x01, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ]);
    elf.extend_from_slice(&u16::to_le_bytes(2));
    elf.extend_from_slice(&u16::to_le_bytes(0x3E));
    elf.extend_from_slice(&u32::to_le_bytes(1));
    elf.extend_from_slice(&u64::to_le_bytes(VADDR_SEG));
    elf.extend_from_slice(&u64::to_le_bytes(OFF_PROG_HDR));
    elf.extend_from_slice(&u64::to_le_bytes(0));
    elf.extend_from_slice(&u32::to_le_bytes(0));
    elf.extend_from_slice(&u16::to_le_bytes(64));
    elf.extend_from_slice(&u16::to_le_bytes(56));
    elf.extend_from_slice(&u16::to_le_bytes(1));
    elf.extend_from_slice(&u16::to_le_bytes(0));
    elf.extend_from_slice(&u16::to_le_bytes(0));
    elf.extend_from_slice(&u16::to_le_bytes(0));

    while elf.len() < OFF_PROG_HDR as usize {
        elf.push(0);
    }

    let p_type   = 1u32;
    let p_flags  = 5u32;
    let p_offset = OFF_SEG;
    let p_vaddr  = VADDR_SEG;
    let p_paddr  = VADDR_SEG;
    let p_filesz = seg.len() as u64;
    let p_memsz  = p_filesz;
    let p_align  = 0x1000u64;

    elf.extend_from_slice(&u32::to_le_bytes(p_type));
    elf.extend_from_slice(&u32::to_le_bytes(p_flags));
    elf.extend_from_slice(&u64::to_le_bytes(p_offset));
    elf.extend_from_slice(&u64::to_le_bytes(p_vaddr));
    elf.extend_from_slice(&u64::to_le_bytes(p_paddr));
    elf.extend_from_slice(&u64::to_le_bytes(p_filesz));
    elf.extend_from_slice(&u64::to_le_bytes(p_memsz));
    elf.extend_from_slice(&u64::to_le_bytes(p_align));

    while elf.len() < OFF_SEG as usize {
        elf.push(0);
    }

    elf.extend_from_slice(&seg);

    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o755)
        .open(out_path)?;
    f.write_all(&elf)?;
    f.flush()?;
    Ok(())
}

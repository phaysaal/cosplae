use std::fs::{File, OpenOptions};
use std::io::{Write, Seek, SeekFrom};
use std::os::unix::fs::OpenOptionsExt; // for mode()
use std::path::Path;

/// Writes a native ELF64 Linux executable at `out_path` that prints "Hello\n" and exits(0).
pub fn emit_min_elf_hello<P: AsRef<Path>>(out_path: P) -> std::io::Result<()> {
    // ---- ELF layout plan ----------------------------------------------------
    // File offsets (hex):
    //   0x0000  ELF header (64 bytes)
    //   0x0040  Program header (56 bytes)
    //   0x1000  Code (.text) + Data (.rodata)
    //
    // Virtual addresses mirror file offsets with base 0x400000:
    const BASE_VADDR: u64 = 0x400000;
    const OFF_ELF_HDR: u64 = 0x0000;
    const OFF_PROG_HDR: u64 = 0x0040;
    const OFF_SEG: u64 = 0x1000;
    const VADDR_SEG: u64 = BASE_VADDR + OFF_SEG;

    // ---------- Machine code -------------------------------------------------
    // _start:
    //   mov rax, 1          ; sys_write
    //   mov rdi, 1          ; fd = 1 (stdout)
    //   lea rsi, [rip+msg]  ; buffer addr
    //   mov rdx, 6          ; length
    //   syscall
    //   mov rax, 60         ; sys_exit
    //   xor rdi, rdi        ; status = 0
    //   syscall
    //
    // Encodings (RIP-rel for lea will be patched):
    let mut code: Vec<u8> = vec![
        0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00,       // mov rax, 1
        0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00,       // mov rdi, 1
        0x48, 0x8D, 0x35, 0, 0, 0, 0,                   // lea rsi, [rip+disp32]  <-- patch
        0x48, 0xC7, 0xC2, 0x06, 0x00, 0x00, 0x00,       // mov rdx, 6
        0x0F, 0x05,                                     // syscall
        0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00,       // mov rax, 60
        0x48, 0x31, 0xFF,                               // xor rdi, rdi
        0x0F, 0x05,                                     // syscall
    ];
    let lea_disp32_offset_in_code = 12 + 3; // index where disp32 bytes start in the code vec

    // The message in .rodata (right after code)
    let msg = b"Hello\n";

    // Compute RIP-relative displacement for LEA:
    // disp32 = (addr(msg) - addr(next_instruction))
    // file layout: [code][msg]
    let code_start_file_off = OFF_SEG as usize;
    let msg_file_off = code_start_file_off + code.len();
    let lea_next_ip_file_off = code_start_file_off + (12 + 7); // at end of LEA instruction
    let disp = (msg_file_off as i64) - (lea_next_ip_file_off as i64);
    let disp_bytes = (disp as i32).to_le_bytes();
    code[lea_disp32_offset_in_code + 0] = disp_bytes[0];
    code[lea_disp32_offset_in_code + 1] = disp_bytes[1];
    code[lea_disp32_offset_in_code + 2] = disp_bytes[2];
    code[lea_disp32_offset_in_code + 3] = disp_bytes[3];

    // Concatenate text+rodata blob
    let mut seg: Vec<u8> = Vec::with_capacity(code.len() + msg.len());
    seg.extend_from_slice(&code);
    seg.extend_from_slice(msg);

    // ---- ELF header (64 bytes) ---------------------------------------------
    // e_ident
    let mut elf: Vec<u8> = Vec::with_capacity(0x1000 + seg.len());
    elf.extend_from_slice(&[
        0x7F, b'E', b'L', b'F',   // EI_MAG
        0x02,                     // EI_CLASS = ELFCLASS64
        0x01,                     // EI_DATA = little-endian
        0x01,                     // EI_VERSION
        0x00,                     // EI_OSABI = System V
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // EI_PAD (7 bytes)
    ]);
    // e_type (ET_EXEC), e_machine (EM_X86_64), e_version
    elf.extend_from_slice(&u16::to_le_bytes(2));       // e_type = 2 (ET_EXEC)
    elf.extend_from_slice(&u16::to_le_bytes(0x3E));    // e_machine = 62 (x86-64)
    elf.extend_from_slice(&u32::to_le_bytes(1));       // e_version = EV_CURRENT

    // e_entry
    let entry = VADDR_SEG; // entry at start of segment (code begins there)
    elf.extend_from_slice(&u64::to_le_bytes(entry));

    // e_phoff (program header offset), e_shoff (no sections -> 0)
    elf.extend_from_slice(&u64::to_le_bytes(OFF_PROG_HDR));
    elf.extend_from_slice(&u64::to_le_bytes(0)); // e_shoff

    // e_flags
    elf.extend_from_slice(&u32::to_le_bytes(0));

    // e_ehsize, e_phentsize, e_phnum, e_shentsize, e_shnum, e_shstrndx
    elf.extend_from_slice(&u16::to_le_bytes(64));  // e_ehsize
    elf.extend_from_slice(&u16::to_le_bytes(56));  // e_phentsize
    elf.extend_from_slice(&u16::to_le_bytes(1));   // e_phnum (1 segment)
    elf.extend_from_slice(&u16::to_le_bytes(0));   // e_shentsize
    elf.extend_from_slice(&u16::to_le_bytes(0));   // e_shnum
    elf.extend_from_slice(&u16::to_le_bytes(0));   // e_shstrndx

    // Pad to program header offset (0x40)
    while elf.len() < OFF_PROG_HDR as usize {
        elf.push(0);
    }

    // ---- Program header (56 bytes) -----------------------------------------
    let p_type   = 1u32; // PT_LOAD
    let p_flags  = 5u32; // R | X  (read + execute)
    let p_offset = OFF_SEG;
    let p_vaddr  = VADDR_SEG;
    let p_paddr  = VADDR_SEG;
    let p_filesz = seg.len() as u64;
    let p_memsz  = p_filesz; // no bss
    let p_align  = 0x1000u64;

    elf.extend_from_slice(&u32::to_le_bytes(p_type));
    elf.extend_from_slice(&u32::to_le_bytes(p_flags));
    elf.extend_from_slice(&u64::to_le_bytes(p_offset));
    elf.extend_from_slice(&u64::to_le_bytes(p_vaddr));
    elf.extend_from_slice(&u64::to_le_bytes(p_paddr));
    elf.extend_from_slice(&u64::to_le_bytes(p_filesz));
    elf.extend_from_slice(&u64::to_le_bytes(p_memsz));
    elf.extend_from_slice(&u64::to_le_bytes(p_align));

    // ---- Pad to segment start (0x1000) -------------------------------------
    while elf.len() < OFF_SEG as usize {
        elf.push(0);
    }

    // ---- Append code+data segment ------------------------------------------
    elf.extend_from_slice(&seg);

    // ---- Write file and mark executable ------------------------------------
    let mut f = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o755) // rwxr-xr-x
        .open(out_path)?;
    f.write_all(&elf)?;
    f.flush()?;
    f.seek(SeekFrom::Start(0))?;
    Ok(())
}

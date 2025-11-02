use std::fs::{File, OpenOptions};
use std::io::{Write, Seek, SeekFrom};
use std::os::unix::fs::OpenOptionsExt; // for mode()
use std::path::Path;

/// Writes a native ELF64 Linux executable at `out_path` that prints the given message and exits(0).
pub fn emit_min_elf_hello<P: AsRef<Path>>(out_path: P, message: &str) -> std::io::Result<()> {
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
    // The message in .rodata (will be placed right after code)
    let msg = message.as_bytes();
    let msg_len = msg.len();

    let mut code: Vec<u8> = vec![
        0x48, 0xC7, 0xC0, 0x01, 0x00, 0x00, 0x00,       // mov rax, 1
        0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00,       // mov rdi, 1
        0x48, 0x8D, 0x35, 0, 0, 0, 0,                   // lea rsi, [rip+disp32]  <-- patch
        0x48, 0xC7, 0xC2, 0, 0, 0, 0,                   // mov rdx, msg_len  <-- patch
        0x0F, 0x05,                                     // syscall
        0x48, 0xC7, 0xC0, 0x3C, 0x00, 0x00, 0x00,       // mov rax, 60
        0x48, 0x31, 0xFF,                               // xor rdi, rdi
        0x0F, 0x05,                                     // syscall
    ];
    let lea_disp32_offset_in_code = 14 + 3; // index where disp32 bytes start in the code vec (LEA starts at byte 14)
    let rdx_len_offset_in_code = 21 + 3;    // index where msg_len bytes start in the code vec (MOV RDX starts at byte 21)

    // Patch the message length into mov rdx instruction
    let len_bytes = (msg_len as u32).to_le_bytes();
    code[rdx_len_offset_in_code + 0] = len_bytes[0];
    code[rdx_len_offset_in_code + 1] = len_bytes[1];
    code[rdx_len_offset_in_code + 2] = len_bytes[2];
    code[rdx_len_offset_in_code + 3] = len_bytes[3];

    // Compute RIP-relative displacement for LEA:
    // disp32 = (addr(msg) - addr(next_instruction))
    // file layout: [code][msg]
    let code_start_file_off = OFF_SEG as usize;
    let msg_file_off = code_start_file_off + code.len();
    let lea_next_ip_file_off = code_start_file_off + (14 + 7); // at end of LEA instruction (LEA starts at byte 14)
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
    // 1. ELF header (64 bytes)
    elf.extend_from_slice(&[
        0x7F, b'E', b'L', b'F', 0x02, 0x01, 0x01, 0x00,
        0,0,0,0,0,0,0,0,
    ]);
    elf.extend_from_slice(&u16::to_le_bytes(2));    // e_type = ET_EXEC
    elf.extend_from_slice(&u16::to_le_bytes(0x3E)); // e_machine = EM_X86_64
    elf.extend_from_slice(&u32::to_le_bytes(1));    // e_version
    elf.extend_from_slice(&u64::to_le_bytes(0x401000)); // e_entry
    elf.extend_from_slice(&u64::to_le_bytes(0x40));     // e_phoff
    elf.extend_from_slice(&u64::to_le_bytes(0));        // e_shoff
    elf.extend_from_slice(&u32::to_le_bytes(0));        // e_flags
    elf.extend_from_slice(&u16::to_le_bytes(64));       // e_ehsize
    elf.extend_from_slice(&u16::to_le_bytes(56));       // e_phentsize
    elf.extend_from_slice(&u16::to_le_bytes(1));        // e_phnum
    elf.extend_from_slice(&u16::to_le_bytes(0));        // e_shentsize
    elf.extend_from_slice(&u16::to_le_bytes(0));        // e_shnum
    elf.extend_from_slice(&u16::to_le_bytes(0));        // e_shstrndx

    // 2. Pad to program header offset (0x40)
    while elf.len() < 0x40 { elf.push(0); }

    // 3. Program header (56 bytes)
    elf.extend_from_slice(&u32::to_le_bytes(1));        // PT_LOAD
    elf.extend_from_slice(&u32::to_le_bytes(5));        // R | X
    elf.extend_from_slice(&u64::to_le_bytes(0x1000));   // p_offset
    elf.extend_from_slice(&u64::to_le_bytes(0x401000)); // p_vaddr
    elf.extend_from_slice(&u64::to_le_bytes(0x401000)); // p_paddr
    elf.extend_from_slice(&u64::to_le_bytes(seg.len() as u64)); // p_filesz
    elf.extend_from_slice(&u64::to_le_bytes(seg.len() as u64)); // p_memsz
    elf.extend_from_slice(&u64::to_le_bytes(0x1000));   // p_align

    // 4. Pad to 0x1000 before writing code
    while elf.len() < 0x1000 { elf.push(0); }

    // 5. Append code+data
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

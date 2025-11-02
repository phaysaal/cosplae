use std::fs::OpenOptions;
use std::io::{Seek, SeekFrom, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

const BASE_VADDR: u64 = 0x400000;
const OFF_ELF_HDR: u64 = 0x0000;
const OFF_PROG_HDR: u64 = 0x0040;
const OFF_SEG: u64 = 0x1000;
const VADDR_SEG: u64 = BASE_VADDR + OFF_SEG;

pub struct ELFBuilder {
    pub code: Vec<u8>,
    pub data: Vec<u8>,
}

impl ELFBuilder {
    pub fn new() -> Self {
        Self { code: Vec::new(), data: Vec::new() }
    }

    pub fn append_code(&mut self, bytes: &[u8]) {
        self.code.extend_from_slice(bytes);
    }

    pub fn append_str(&mut self, s: &str) -> usize {
        let offset = self.data.len();
        self.data.extend_from_slice(s.as_bytes());
        offset
    }

    /// Writes a valid ELF64 executable with one LOAD segment at 0x401000
    pub fn emit<P: AsRef<Path>>(&self, out_path: P) -> std::io::Result<()> {
        // ---- Segment: code + data ----
        let mut seg = Vec::with_capacity(self.code.len() + self.data.len());
        seg.extend_from_slice(&self.code);
        seg.extend_from_slice(&self.data);

        // ---- ELF header ----
        let mut elf: Vec<u8> = Vec::with_capacity(0x1000 + seg.len());

        // e_ident
        elf.extend_from_slice(&[
            0x7F, b'E', b'L', b'F',
            0x02, // ELFCLASS64
            0x01, // little endian
            0x01, // version
            0x00, // System V
            0, 0, 0, 0, 0, 0, 0, 0,
        ]);

        // e_type, e_machine, e_version
        elf.extend_from_slice(&u16::to_le_bytes(2));    // ET_EXEC
        elf.extend_from_slice(&u16::to_le_bytes(0x3E)); // EM_X86_64
        elf.extend_from_slice(&u32::to_le_bytes(1));    // EV_CURRENT

        // e_entry
        elf.extend_from_slice(&u64::to_le_bytes(VADDR_SEG)); // entry = start of segment
        elf.extend_from_slice(&u64::to_le_bytes(OFF_PROG_HDR)); // e_phoff
        elf.extend_from_slice(&u64::to_le_bytes(0));           // e_shoff
        elf.extend_from_slice(&u32::to_le_bytes(0));           // e_flags
        elf.extend_from_slice(&u16::to_le_bytes(64));          // e_ehsize
        elf.extend_from_slice(&u16::to_le_bytes(56));          // e_phentsize
        elf.extend_from_slice(&u16::to_le_bytes(1));           // e_phnum
        elf.extend_from_slice(&u16::to_le_bytes(0));           // e_shentsize
        elf.extend_from_slice(&u16::to_le_bytes(0));           // e_shnum
        elf.extend_from_slice(&u16::to_le_bytes(0));           // e_shstrndx

        // Pad to 0x40
        while elf.len() < OFF_PROG_HDR as usize {
            elf.push(0);
        }

        // ---- Program header ----
        let p_type = 1u32;  // PT_LOAD
        let p_flags = 5u32; // R | X
        let p_offset = OFF_SEG;
        let p_vaddr = VADDR_SEG;
        let p_paddr = VADDR_SEG;
        let p_filesz = seg.len() as u64;
        let p_memsz = p_filesz;
        let p_align = 0x1000u64;

        elf.extend_from_slice(&u32::to_le_bytes(p_type));
        elf.extend_from_slice(&u32::to_le_bytes(p_flags));
        elf.extend_from_slice(&u64::to_le_bytes(p_offset));
        elf.extend_from_slice(&u64::to_le_bytes(p_vaddr));
        elf.extend_from_slice(&u64::to_le_bytes(p_paddr));
        elf.extend_from_slice(&u64::to_le_bytes(p_filesz));
        elf.extend_from_slice(&u64::to_le_bytes(p_memsz));
        elf.extend_from_slice(&u64::to_le_bytes(p_align));

        // Pad to 0x1000 before segment
        while elf.len() < OFF_SEG as usize {
            elf.push(0);
        }

        // ---- Append segment ----
        elf.extend_from_slice(&seg);

        // ---- Write file ----
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o755)
            .open(out_path)?;
        f.write_all(&elf)?;
        f.flush()?;
        f.seek(SeekFrom::Start(0))?;
        Ok(())
    }
}

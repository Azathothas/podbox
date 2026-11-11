//! The one question the packager asks of an ELF file: does it name a
//! program interpreter (PT_INTERP)? A static release binary has none.
//! Both classes and both byte orders, so each arch of the nightly reads.

const PT_INTERP: u32 = 3;

/// `Ok(true)` with a PT_INTERP, `Ok(false)` without one, `Err` when the
/// bytes are not a well-formed ELF header and program header table.
pub(crate) fn has_interp(b: &[u8]) -> Result<bool, &'static str> {
    if b.len() < 52 || &b[..4] != b"\x7fELF" {
        return Err("no ELF magic");
    }
    let wide = match b[4] {
        1 => false,
        2 => true,
        _ => return Err("unknown ELF class"),
    };
    let big = match b[5] {
        1 => false,
        2 => true,
        _ => return Err("unknown ELF byte order"),
    };
    let u16_at = |o: usize| -> Result<u64, &'static str> {
        let s: [u8; 2] = b.get(o..o + 2).ok_or("short header")?.try_into().unwrap();
        Ok(u64::from(if big {
            u16::from_be_bytes(s)
        } else {
            u16::from_le_bytes(s)
        }))
    };
    let u32_at = |o: usize| -> Result<u64, &'static str> {
        let s: [u8; 4] = b.get(o..o + 4).ok_or("short header")?.try_into().unwrap();
        Ok(u64::from(if big {
            u32::from_be_bytes(s)
        } else {
            u32::from_le_bytes(s)
        }))
    };
    let u64_at = |o: usize| -> Result<u64, &'static str> {
        let s: [u8; 8] = b.get(o..o + 8).ok_or("short header")?.try_into().unwrap();
        Ok(if big {
            u64::from_be_bytes(s)
        } else {
            u64::from_le_bytes(s)
        })
    };
    let (phoff, phentsize, phnum) = if wide {
        (u64_at(32)?, u16_at(54)?, u16_at(56)?)
    } else {
        (u32_at(28)?, u16_at(42)?, u16_at(44)?)
    };
    if phnum == 0 {
        return Err("no program headers");
    }
    if phentsize < 4 {
        return Err("short program header entry");
    }
    for i in 0..phnum {
        let at = phoff
            .checked_add(i * phentsize)
            .and_then(|v| usize::try_from(v).ok())
            .ok_or("program header out of range")?;
        if at
            .checked_add(phentsize as usize)
            .is_none_or(|end| end > b.len())
        {
            return Err("program header out of range");
        }
        if u32_at(at)? == PT_INTERP as u64 {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::has_interp;

    #[test]
    fn this_static_test_binary_has_no_interpreter() {
        let me = std::fs::read(std::env::current_exe().unwrap()).unwrap();
        assert_eq!(has_interp(&me), Ok(false));
    }

    #[test]
    fn text_is_not_an_elf_file() {
        assert!(has_interp(b"invalid ELF fixture\n").is_err());
        assert!(has_interp(&[0x7f, b'E', b'L', b'F']).is_err());
    }

    /// A 64-bit little-endian header with one PT_INTERP program header,
    /// and the same header with PT_LOAD.
    fn header(p_type: u32) -> Vec<u8> {
        let mut b = vec![0u8; 64 + 56];
        b[..4].copy_from_slice(b"\x7fELF");
        b[4] = 2;
        b[5] = 1;
        b[32..40].copy_from_slice(&64u64.to_le_bytes());
        b[54..56].copy_from_slice(&56u16.to_le_bytes());
        b[56..58].copy_from_slice(&1u16.to_le_bytes());
        b[64..68].copy_from_slice(&p_type.to_le_bytes());
        b
    }

    #[test]
    fn a_pt_interp_header_is_found_and_a_load_header_is_not() {
        assert_eq!(has_interp(&header(3)), Ok(true));
        assert_eq!(has_interp(&header(1)), Ok(false));
    }

    #[test]
    fn a_table_past_the_end_is_refused() {
        let mut b = header(1);
        b.truncate(80);
        assert!(has_interp(&b).is_err());
    }
}

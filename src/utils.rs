use std::io::Read;

use color_eyre::Result;

pub fn count_lines<R: Read>(reader: &mut R) -> Result<usize> {
    let mut buf = [0u8; 32 * 1024];
    let mut count = 0;

    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        count += buf[..n].iter().filter(|&&b| b == b'\n').count();
    }

    Ok(count)
}

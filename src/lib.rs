use std::{
    fs::File,
    io::{self, BufReader, Read},
    path::Path,
};

use crc32fast::Hasher;

pub fn calculate_crc32(
    path: impl AsRef<Path>,
    mut on_progress: impl FnMut(f32),
) -> Result<u32, io::Error> {
    let file = File::open(path)?;
    let size = file.metadata()?.len();

    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();
    let mut buffer = [0; 4096];
    let mut total_read = 0;
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        total_read += read;
        hasher.update(&buffer[..read]);
        on_progress((total_read as f32 / size as f32) * 100.0)
    }

    Ok(hasher.finalize())
}

/// reference: https://sar.informatik.hu-berlin.de/research/publications/SAR-PR-2006-05/SAR-PR-2006-05_.pdf
pub fn calculate_new_bytes(from: u32, to: u32) -> [u8; 4] {
    const CRC_POLY: u32 = 0xEDB88320;
    const CRC_INV: u32 = 0x5B358FD3;
    const FINAL_XOR: u32 = 0xFFFFFFFF;

    let mut content = 0;
    let mut target = to ^ FINAL_XOR;
    for _ in 0..32 {
        if content & 1 != 0 {
            content = (content >> 1) ^ CRC_POLY;
        } else {
            content >>= 1;
        }

        if target & 1 != 0 {
            content ^= CRC_INV;
        }

        target >>= 1;
    }

    content ^= from ^ FINAL_XOR;
    let mut bytes = [0; 4];
    for (idx, value) in bytes.iter_mut().enumerate() {
        *value = (content >> (idx * 8)) as u8;
    }

    bytes
}

#[cfg(test)]
mod tests {
    use crc32fast::Hasher;
    use rayon::prelude::{IntoParallelIterator, ParallelIterator};

    use super::calculate_new_bytes;

    #[test]
    fn calculates_new_bytes_correctly() {
        (0..1_000_000).into_par_iter().for_each(|_| {
            let (from, to) = (fastrand::u32(..), fastrand::u32(..));

            let mut hasher = Hasher::new_with_initial(from);
            let new_bytes = calculate_new_bytes(from, to);
            hasher.update(&new_bytes);

            assert_eq!(hasher.finalize(), to);
        });
    }
}

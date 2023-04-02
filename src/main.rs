use std::{
    error::Error,
    fs::File,
    io::{self, BufReader, Read, Write},
};

use crc32fast::Hasher;
use kdam::{term::Colorizer, BarExt};

const GRADIENT: (&str, &str) = ("#57ebde", "#aefb2a");

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();

    let old_crc32 = calculate_crc32_from_file(&args.filename)?;

    if old_crc32 == args.new_crc32 {
        return Err(format!(
            "File `{}` already has `{:08X}` CRC32 hash",
            &args.filename, args.new_crc32
        )
        .into());
    }

    let new_bytes = calculate_new_bytes(old_crc32, args.new_crc32);
    let calculated_new_crc32 = {
        let mut hasher = Hasher::new_with_initial(old_crc32);
        hasher.update(&new_bytes);
        hasher.finalize()
    };

    if calculated_new_crc32 != args.new_crc32 {
        return Err(format!(
            "Got incorrect CRC32 hash `{:08X}`, expected `{:08X}`",
            calculated_new_crc32, args.new_crc32
        )
        .into());
    }

    let prompt = format!(
        "Got the correct CRC32 hash `{:08X}`, Do you want to patch the file? [Y/n]: ",
        calculated_new_crc32
    )
    .colorize(GRADIENT.0);
    if args.execute || matches!(rprompt::prompt_reply(prompt)?.as_str(), "Y" | "y" | "") {
        let mut file = File::options().append(true).open(&args.filename)?;
        file.write_all(&new_bytes)?;
        file.flush()?;
    }

    Ok(())
}

#[derive(argh::FromArgs)]
/// Modify CRC32 of a file
struct Args {
    /// the file to modify
    #[argh(positional)]
    filename: String,

    /// the CRC32 hash to modify to
    #[argh(positional, from_str_fn(parse_u32_hex))]
    new_crc32: u32,

    /// don't prompt when patching the file
    #[argh(short = 'x', switch)]
    execute: bool,
}

fn calculate_crc32_from_file(filename: &str) -> Result<u32, io::Error> {
    let file = File::open(filename)?;

    let mut pb = kdam::tqdm!(
        total = file.metadata()?.len() as usize,
        bar_format = format!("{{animation}} {}", "{percentage:3.0}%".colorize(GRADIENT.0)),
        colour = format!("gradient({},{})", GRADIENT.1, GRADIENT.0)
    );

    pb.write(format!("Calculating CRC32 hash of `{}`", filename).colorize(GRADIENT.0));

    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();
    let mut buffer = [0; 4096];
    loop {
        let read = reader.read(&mut buffer)?;
        if read == 0 {
            break;
        }

        hasher.update(&buffer[..read]);
        pb.update(read);
    }

    Ok(hasher.finalize())
}

/// reference: https://sar.informatik.hu-berlin.de/research/publications/SAR-PR-2006-05/SAR-PR-2006-05_.pdf
fn calculate_new_bytes(from: u32, to: u32) -> [u8; 4] {
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

fn parse_u32_hex(value: &str) -> Result<u32, String> {
    u32::from_str_radix(value, 16).map_err(|err| err.to_string())
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

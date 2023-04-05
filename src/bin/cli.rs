use std::{error::Error, fs::File, io::Write};

use crc32fast::Hasher;
use kdam::{term::Colorizer, BarExt};

use modify_crc32::{calculate_crc32, calculate_new_bytes};

const GRADIENT: (&str, &str) = ("#57ebde", "#aefb2a");

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();

    let old_crc32 = {
        let mut pb = kdam::tqdm!(
            total = 10000,
            bar_format = format!("{{animation}} {}", "{percentage:3.0}%".colorize(GRADIENT.0)),
            colour = format!("gradient({},{})", GRADIENT.1, GRADIENT.0)
        );

        pb.write(format!("Calculating CRC32 hash of `{}`", &args.filename).colorize(GRADIENT.0));
        calculate_crc32(&args.filename, |progress| {
            pb.update_to((progress * 100.0) as usize);
        })?
    };

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

fn parse_u32_hex(value: &str) -> Result<u32, String> {
    u32::from_str_radix(value, 16).map_err(|err| err.to_string())
}
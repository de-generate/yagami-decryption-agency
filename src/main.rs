use std::{convert::TryInto, fs, io::Read, path::PathBuf};

use clap::{CommandFactory, Parser, ValueEnum};
use dialoguer::{Confirm, Select};
use indicatif::{ProgressBar, ProgressStyle};
use spinners::{Spinner, Spinners};

const CHARA_KEY: &'static [u8; 512] = include_bytes!("keys/chara_key.bin");
const CHARA2_KEY: &'static [u8; 512] = include_bytes!("keys/chara2_key.bin");

#[derive(Parser)]
#[clap(name = "yagami-decryption-agency")]
#[clap(author = "SutandoTsukai181")]
#[clap(version = "0.1.0")]
#[clap(about = "Decrypts/encrypts Judgment and Lost Judgment PC chara.par archives", long_about = None)]
struct Args {
    /// Path to input file.
    #[clap(value_parser)]
    input: PathBuf,

    /// Path to output file. Defaults to input with ".decrypted.par" as the extension.
    #[clap(value_parser)]
    output: Option<PathBuf>,

    /// Operation mode.
    #[clap(value_enum, value_parser, default_value = "auto")]
    mode: Mode,

    /// Type of the encrypted PAR file.
    #[clap(value_enum, value_parser, default_value = "auto")]
    par_type: ParType,

    /// Overwrite files without asking.
    #[clap(short, long, action)]
    overwrite: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Mode {
    /// Automatically select mode based on input file name.
    Auto,

    /// Decrypt file.
    Decrypt,

    /// Encrypt file.
    Encrypt,
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum ParType {
    /// Automatically select PAR type based on its contents.
    Auto,

    /// chara.par.
    Chara,

    /// chara2.par (Lost Judgment only).
    Chara2,
}

fn xor(data: &mut Vec<u8>, key: &[u8; 512]) {
    let mut key = key.iter().cycle();

    println!("Performing XOR...");
    let bar = ProgressBar::new(data.len() as u64)
        .with_style(ProgressStyle::with_template("{bar:50.cyan/blue} [{percent}%]").unwrap());

    for (i, byte) in data.iter_mut().enumerate() {
        if i % (1024 * 1024) == 0 {
            bar.inc(1024 * 1024);
        }

        *byte ^= key.next().unwrap();
    }

    bar.finish();
    println!("\n");
}

fn rotate<const LEFT: bool>(data: &mut Vec<u8>) {
    println!("Rotating bits...");
    let bar = ProgressBar::new(data.len() as u64)
        .with_style(ProgressStyle::with_template("{bar:50.cyan/blue} [{percent}%]").unwrap());

    for (i, chunk) in data.chunks_mut(8).enumerate() {
        if (i % (1024 * 1024 / 8)) == 0 {
            bar.inc(1024 * 1024);
        }

        let value = u64::from_le_bytes(chunk.try_into().unwrap());
        let value_rotated = if LEFT {
            value.rotate_left((i % 64) as u32)
        } else {
            value.rotate_left((i % 64) as u32)
        };

        chunk.copy_from_slice(&value_rotated.to_le_bytes());
    }

    bar.finish();
    println!("\n");
}

fn rotate_left(data: &mut Vec<u8>) {
    rotate::<true>(data)
}

fn rotate_right(data: &mut Vec<u8>) {
    rotate::<false>(data)
}

fn pad(data: &mut Vec<u8>) {
    let rem = data.len() % 8;
    if rem != 0 {
        data.resize(data.len() - rem + 8, 0);
    }
}

fn decrypt(mut data: Vec<u8>, key: &[u8; 512]) -> Vec<u8> {
    println!("Decrypting...\n");

    xor(&mut data, key);
    pad(&mut data);
    rotate_left(&mut data);

    data
}

fn encrypt(mut data: Vec<u8>, key: &[u8; 512]) -> Vec<u8> {
    println!("Encrypting...\n");

    pad(&mut data);
    rotate_right(&mut data);
    xor(&mut data, key);

    data
}

fn main() {
    let mut args = Args::parse();

    // Print header
    print!(
        "{}{}\n",
        Args::command().render_version(),
        Args::command().get_author().unwrap()
    );

    if let Mode::Auto = args.mode {
        let file_name = args
            .input
            .file_name()
            .expect("Invalid path")
            .to_str()
            .unwrap_or_default();

        if file_name.ends_with(".decrypted.par") {
            args.mode = Mode::Encrypt;
        } else if file_name.ends_with(".par") {
            args.mode = Mode::Decrypt;
        } else {
            println!("Unable to determine operation mode.");
            println!("Select a mode:");
            args.mode = match Select::new()
                .items(&["Encrypt", "Decrypt"])
                .clear(false)
                .interact()
                .expect("Operation mode needs to be selected")
            {
                0 => Mode::Encrypt,
                1 => Mode::Decrypt,
                _ => panic!("Unexpected selection."),
            };
        }
    }

    let mut sp = Spinner::new(Spinners::Line, "Reading file...".into());
    let par = fs::read(&args.input).expect("Could not read file");
    sp.stop_with_newline();

    let key = match args.par_type {
        ParType::Auto => match &par[0..4] {
            b"\xAC\xC5\x8B\x99" => CHARA_KEY,
            b"\x01\x6E\x58\xE4" => CHARA2_KEY,
            _ => {
                println!();
                println!("Unable to determine PAR type.");
                println!("Select a type:");
                match Select::new()
                    .items(&["chara.par", "chara2.par"])
                    .clear(false)
                    .interact()
                    .expect("PAR type needs to be selected")
                {
                    0 => CHARA_KEY,
                    1 => CHARA2_KEY,
                    _ => panic!("Unexpected selection."),
                }
            }
        },
        ParType::Chara => CHARA_KEY,
        ParType::Chara2 => CHARA2_KEY,
    };

    let (result, output_extension) = match args.mode {
        Mode::Decrypt => (decrypt(par, key), "decrypted.par"),
        Mode::Encrypt => (encrypt(par, key), "par"),
        _ => unreachable!(),
    };

    let output = match args.output {
        Some(output) => output,
        None => {
            let mut output = args.input.clone();

            if args.mode == Mode::Encrypt && output.extension().is_some() {
                output.set_file_name(
                    output
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace(".decrypted.par", ".par"),
                );
            }

            output.set_extension(output_extension);
            output
        }
    };

    println!("Writing file to {:?}", &output);

    if !args.overwrite
        && output.is_file()
        && !Confirm::new()
            .with_prompt("File already exists. Overwrite?")
            .interact()
            .unwrap_or(false)
    {
        println!("Aborting.");
        return;
    }

    println!();

    let mut sp = Spinner::new(Spinners::Line, "Writing file...".into());
    fs::write(&output, result).expect("Could not write file");
    sp.stop_with_newline();

    println!();
    println!("Finished.");
    println!("Press ENTER to continue...");
    std::io::stdin().read(&mut [0]).unwrap();
}

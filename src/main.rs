#![feature(iter_array_chunks)]

mod array_chunks_pad;
use array_chunks_pad::ArrayChunksPadExtension;

use bytemuck::cast_slice;
use clap::{CommandFactory, Parser, ValueEnum};
use dialoguer::{Confirm, Select};
use spinners::{Spinner, Spinners};
use std::{
    fs::File,
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

const CHARA_KEY: &'static [u8; 512] = include_bytes!("keys/chara_key.bin");
const CHARA2_KEY: &'static [u8; 512] = include_bytes!("keys/chara2_key.bin");

const READ_BUFFER_SIZE: usize = 8 * 1024 * 1024;
const WRITE_BUFFER_SIZE: usize = 8 * 1024 * 1024;

#[derive(Parser)]
#[command(author, version, about)]
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

    /// Skip asking to press ENTER when done.
    #[clap(short, long, action)]
    quick_exit: bool,
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

fn encrypt<R, W, K>(reader: R, mut writer: W, mut key: K)
where
    R: Read,
    W: Write,
    K: Iterator<Item = &'static u64>,
{
    for val in reader
        .bytes()
        .map(|byte| byte.unwrap())
        .array_chunks_pad::<8>(0)
        .enumerate()
        .map(|(i, bytes)| {
            u64::from_le_bytes(bytes).rotate_right((i % 64) as u32) ^ key.next().unwrap()
        })
    {
        writer.write(&val.to_le_bytes()).unwrap();
    }
}

fn decrypt<R, W, K>(reader: R, mut writer: W, mut key: K)
where
    R: Read,
    W: Write,
    K: Iterator<Item = &'static u64>,
{
    for val in reader
        .bytes()
        .map(|byte| byte.unwrap())
        .array_chunks_pad::<8>(0)
        .enumerate()
        .map(|(i, bytes)| {
            (u64::from_le_bytes(bytes) ^ key.next().unwrap()).rotate_left((i % 64) as u32)
        })
    {
        writer.write(&val.to_le_bytes()).unwrap();
    }
}

fn main() {
    let args = Args::parse();

    println!(
        "{}{}\n",
        Args::command().render_version(),
        Args::command().get_author().unwrap()
    );

    let input = args.input;

    let mode = if args.mode == Mode::Auto {
        let input_file_name = input
            .file_name()
            .expect("Invalid inputpath")
            .to_str()
            .unwrap_or_default();

        if input_file_name.ends_with(".decrypted.par") {
            Mode::Encrypt
        } else if input_file_name.ends_with(".par") {
            Mode::Decrypt
        } else {
            match Select::new()
                .with_prompt("Unable to determine operation mode.\nSelect a mode:")
                .items(&["Encrypt", "Decrypt"])
                .clear(false)
                .interact()
                .expect("Operation mode needs to be selected")
            {
                0 => Mode::Encrypt,
                1 => Mode::Decrypt,
                _ => unreachable!(),
            }
        }
    } else {
        args.mode
    };

    let output = match args.output {
        Some(output) => output,
        None => {
            let mut output = input.clone();

            if mode == Mode::Encrypt && output.extension().is_some() {
                output.set_file_name(
                    output
                        .file_stem()
                        .unwrap()
                        .to_str()
                        .unwrap()
                        .replace(".decrypted.par", ".par"),
                );
            }

            output.set_extension(match mode {
                Mode::Encrypt => "par",
                Mode::Decrypt => "decrypted.par",
                Mode::Auto => unreachable!(),
            });

            output
        }
    };

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

    let mut input_file = File::open(&input).unwrap();
    let mut magic_buf = [0; 4];
    input_file.read_exact(&mut magic_buf).unwrap();
    input_file.seek(SeekFrom::Start(0)).unwrap();

    let key = match args.par_type {
        ParType::Chara => CHARA_KEY,
        ParType::Chara2 => CHARA2_KEY,
        ParType::Auto => match &magic_buf {
            b"\xAC\xC5\x8B\x99" => CHARA_KEY,
            b"\x01\x6E\x58\xE4" => CHARA2_KEY,
            _ => {
                match Select::new()
                    .with_prompt("Unable to determine PAR type.\nSelect a type:")
                    .items(&["chara.par", "chara2.par"])
                    .clear(false)
                    .interact()
                    .expect("PAR type needs to be selected")
                {
                    0 => CHARA_KEY,
                    1 => CHARA2_KEY,
                    _ => unreachable!(),
                }
            }
        },
    };

    let mode_text = match mode {
        Mode::Encrypt => "encrypting".to_string(),
        Mode::Decrypt => "decrypting".to_string(),
        _ => unreachable!(),
    };

    println!(
        "
{mode_text} {input:?}
writing output to {output:?}
    "
    );

    let mut spinner = Spinner::new(Spinners::Line, format!("{mode_text}..."));

    let reader = BufReader::with_capacity(READ_BUFFER_SIZE, input_file);
    let writer = BufWriter::with_capacity(WRITE_BUFFER_SIZE, File::create(output).unwrap());
    let key = cast_slice::<_, u64>(key).iter().cycle();

    match mode {
        Mode::Encrypt => encrypt(reader, writer, key),
        Mode::Decrypt => decrypt(reader, writer, key),
        _ => unreachable!(),
    }

    spinner.stop_with_newline();

    println!();
    println!("Finished.");

    if !args.quick_exit {
        println!("Press ENTER to continue...");
        std::io::stdin().read(&mut [0]).unwrap();
    }
}

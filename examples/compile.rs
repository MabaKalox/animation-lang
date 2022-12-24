use animation_lang::program::Program;
use anyhow::{bail, Result};
use clap::Parser;
use reqwest::blocking::Client;
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short)]
    in_file: PathBuf,

    #[arg(long, short)]
    out_file: Option<PathBuf>,

    #[arg(long, short, help = "address to send base64 encoded program")]
    send_addr: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let source_code = std::fs::read_to_string(args.in_file)?;
    let p = Program::from_source(&source_code)?;

    println!("assembly:");
    println!("{:?}", p);

    if let Some(path) = args.out_file {
        println!("Saving program into {:?}", path);
        std::fs::write(path, p.code())?;
    }

    if let Some(addr) = args.send_addr {
        println!("Sending program to {}", addr);
        let resp = Client::new()
            .post(addr)
            .body(base64::encode(p.code()))
            .send()?;

        if resp.status() != 200 {
            bail!("{}", resp.text()?);
        }
    }

    Ok(())
}

use animation_lang::program::Program;
use anyhow::{bail, Result};
use clap::Parser;
use std::{
    fs::File,
    io::{Read, Write},
    net::SocketAddr,
    path::PathBuf,
};

#[derive(Parser, Debug)]
struct Args {
    #[arg(long, short)]
    in_file: PathBuf,

    #[arg(long, short)]
    out_file: Option<PathBuf>,

    #[arg(long, short)]
    send_addr: Option<SocketAddr>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let sourse_code = std::fs::read_to_string(args.in_file)?;
    let p = Program::from_source(&sourse_code)
        .or_else(|e| bail!(e))
        .unwrap();

    println!("assembly: {:?}", p);

    if let Some(path) = args.out_file {
        println!("Saving program into {:?}", path);
        std::fs::write(path, p.code())?;
    }

    if let Some(addr) = args.send_addr {
        println!("Sending program to {}", addr);
        let mut stream = std::net::TcpStream::connect(addr)?;

        stream.write_all(p.code())?;
        println!("Program send");
    }

    Ok(())
}

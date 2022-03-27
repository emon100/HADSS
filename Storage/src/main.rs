#![feature(once_cell)]
#![feature(iter_intersperse)]
mod server;

use clap::Parser;
use server::init_httpserver;
use std::lazy::SyncLazy;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long, default_value = "0.0.0.0")]
    addr: String,
    #[clap(short, long, default_value_t = 10001)]
    port: u16,
    #[clap(short, long, default_value = "/tmp/storage")]
    storage_location: String,
    #[clap(long, default_value_t = 10)]
    storage_directory_depth: usize,
}

pub static ARGS: SyncLazy<Args> = SyncLazy::new(|| {
    let mut args: Args = Args::parse();
    args.storage_location = args.storage_location.trim_end_matches("/").parse().unwrap();
    args
});

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Listening: {}:{}", ARGS.addr, ARGS.port);
    init_httpserver().await
}

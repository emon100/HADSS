#![feature(once_cell)]
#![feature(iter_intersperse)]

mod app;
mod network;
mod store;
pub mod testing;

use clap::Parser;
use network::init_httpserver;
use std::lazy::SyncLazy;
use std::sync::Arc;
use openraft::{Raft};
use crate::network::ExampleNetwork;
use crate::store::StorageNodeFileStore;
use crate::store::StoreFileRequest;
use crate::store::StoreFileResponse;

pub type StorageNodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub StorageRaftTypeConfig: D = StoreFileRequest, R = StoreFileResponse, NodeId = StorageNodeId
);

pub type ExampleRaft = Raft<StorageRaftTypeConfig, ExampleNetwork, Arc<StorageNodeFileStore>>;

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
    #[clap(short, long, default_value_t = 0)]
    node_id: StorageNodeId,
    #[clap(long, default_value_t = 1<<30)]
    payload_size: usize,
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

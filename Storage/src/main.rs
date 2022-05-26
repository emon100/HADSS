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
use crate::network::StorageNodeNetwork;
use crate::store::StorageNodeFileStore;
use crate::store::StorageNodeRequest;
use crate::store::StorageNodeResponse;

pub type StorageNodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub StorageRaftTypeConfig: D = StorageNodeRequest, R = StorageNodeResponse, NodeId = StorageNodeId
);

pub type ExampleRaft = Raft<StorageRaftTypeConfig, StorageNodeNetwork, Arc<StorageNodeFileStore>>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long, default_value = "0.0.0.0")]
    listen_addr: String, //The ip address server listen to.
    #[clap(short, long, default_value_t = 10001)]
    port: u16,
    #[clap(short, long, default_value = "/tmp/storage")]
    storage_location: String,
    #[clap(short, long)]
    monitor_addr: String, //The ip port of monitor server.
    #[clap(long, default_value_t = 10)]
    storage_directory_depth: usize,
    #[clap(long, default_value_t = 0)]
    node_id: StorageNodeId,
    #[clap(long)]
    node_addr: String, //The ip address for others to connect to this node.
    #[clap(long, default_value_t = 1<<16)] // 64KB of payload per request
    payload_size: usize,
}

pub static ARGS: SyncLazy<Args> = SyncLazy::new(|| {
    let mut args: Args = Args::parse();
    args.storage_location = args.storage_location.trim_end_matches("/").parse().unwrap();
    args
});


//TODO: only the leader know is follower catching log.

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Listening: {}:{}", ARGS.listen_addr, ARGS.port);
    init_httpserver().await
}

#![feature(once_cell)]
#![feature(iter_intersperse)]

use std::fmt::Display;
use std::lazy::SyncLazy;
use std::sync::Arc;

use clap::Parser;
use openraft::{Config, Raft};
use openraft;

use network::init_httpserver;

use crate::network::raft_network_impl::StorageNodeNetwork;
use crate::store::StorageNodeFileStore;
use crate::store::StorageNodeRequest;
use crate::store::StorageNodeResponse;

mod app;
mod network;
mod store;
pub mod testing;

pub type StorageNodeId = u64;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq, Default)]
pub struct StorageNode {
    pub rpc_addr: String,
    pub api_addr: String,
}

impl Display for StorageNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ExampleNode {{ rpc_addr: {}, api_addr: {} }}",
            self.rpc_addr, self.api_addr
        )
    }
}

openraft::declare_raft_types!(
    pub StorageRaftTypeConfig:
        D = StorageNodeRequest, R = StorageNodeResponse, NodeId = StorageNodeId, Node = StorageNode
);
pub type StorageNodeRaft = Raft<StorageRaftTypeConfig, StorageNodeNetwork, Arc<StorageNodeFileStore>>;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    #[clap(short, long, default_value = "0.0.0.0")]
    listen_addr: String,
    //The ip address server listen to.
    #[clap(short, long, default_value_t = 10001)]
    port: u16,
    #[clap(short, long, default_value = "/tmp/storage")]
    storage_location: String,
    #[clap(short, long)]
    monitor_addr: String,
    //The ip port of monitor server.
    #[clap(long, default_value_t = 10)]
    storage_directory_depth: usize,
    #[clap(long, default_value_t = 0)]
    node_id: StorageNodeId,
    #[clap(long)]
    node_addr: String,
    //The ip address for others to connect to this node.
    #[clap(long, default_value_t = 1 << 16)] // 64KB of payload per request
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

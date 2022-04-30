#![feature(once_cell)]
#![feature(iter_intersperse)]

mod app;
mod network;
mod store;

use clap::Parser;
use network::init_httpserver;
use std::lazy::SyncLazy;
use std::sync::Arc;
use openraft::{Raft, StorageError};
use openraft::testing::Suite;
use crate::network::ExampleNetwork;
use crate::store::ExampleStore;
use crate::store::ExampleRequest;
use crate::store::ExampleResponse;

pub type ExampleNodeId = u64;

openraft::declare_raft_types!(
    /// Declare the type configuration for example K/V store.
    pub ExampleTypeConfig: D = ExampleRequest, R = ExampleResponse, NodeId = ExampleNodeId
);

pub type ExampleRaft = Raft<ExampleTypeConfig, ExampleNetwork, Arc<ExampleStore>>;

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
    node_id: ExampleNodeId,
}

pub static ARGS: SyncLazy<Args> = SyncLazy::new(|| {
    let mut args: Args = Args::parse();
    args.storage_location = args.storage_location.trim_end_matches("/").parse().unwrap();
    args
});

pub async fn new_async() -> Arc<ExampleStore> {
    Arc::new(ExampleStore::default())
}

#[test]
pub fn test_mem_store() -> Result<(), StorageError<ExampleNodeId>> {
    Suite::test_all(new_async)?;
    Ok(())
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    println!("Listening: {}:{}", ARGS.addr, ARGS.port);
    init_httpserver().await
}

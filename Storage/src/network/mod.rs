use std::sync::Arc;

use actix_web::{App, HttpServer};
use actix_web::web::Data;
use async_trait::async_trait;
use openraft::{Config, Node, Raft};
use openraft::error::AppendEntriesError;
use openraft::error::InstallSnapshotError;
use openraft::error::NetworkError;
use openraft::error::RemoteError;
use openraft::error::RPCError;
use openraft::error::VoteError;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::AppendEntriesResponse;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::InstallSnapshotResponse;
use openraft::raft::VoteRequest;
use openraft::raft::VoteResponse;
use openraft::RaftNetwork;
use openraft::RaftNetworkFactory;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{ARGS, StorageNodeId, StorageNodeFileStore};
use crate::app::StorageNode;
use crate::StorageRaftTypeConfig;

pub mod slice;
pub mod raft;
pub mod management;

pub struct ExampleNetwork {}

impl ExampleNetwork {
    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: StorageNodeId,
        target_node: Option<&Node>,
        uri: &str,
        req: Req,
    ) -> Result<Resp, RPCError<StorageRaftTypeConfig, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        let addr = target_node.map(|x| &x.addr).unwrap();

        let url = format!("http://{}/{}", addr, uri);
        let client = reqwest::Client::new();

        let resp = client.post(url).json(&req).send().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let res: Result<Resp, Err> = resp.json().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        res.map_err(|e| RPCError::RemoteError(RemoteError::new(target, e)))
    }
}

// NOTE: This could be implemented also on `Arc<ExampleNetwork>`, but since it's empty, implemented directly.
#[async_trait]
impl RaftNetworkFactory<StorageRaftTypeConfig> for ExampleNetwork {
    type Network = ExampleNetworkConnection;

    async fn connect(&mut self, target: StorageNodeId, node: Option<&Node>) -> Self::Network {
        ExampleNetworkConnection {
            owner: ExampleNetwork {},
            target,
            target_node: node.cloned(),
        }
    }
}

pub struct ExampleNetworkConnection {
    owner: ExampleNetwork,
    target: StorageNodeId,
    target_node: Option<Node>,
}

#[async_trait]
impl RaftNetwork<StorageRaftTypeConfig> for ExampleNetworkConnection {
    async fn send_append_entries(
        &mut self,
        req: AppendEntriesRequest<StorageRaftTypeConfig>,
    ) -> Result<AppendEntriesResponse<StorageNodeId>, RPCError<StorageRaftTypeConfig, AppendEntriesError<StorageNodeId>>>
    {
        self.owner.send_rpc(self.target, self.target_node.as_ref(), "raft-append", req).await
    }

    async fn send_install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<StorageRaftTypeConfig>,
    ) -> Result<InstallSnapshotResponse<StorageNodeId>, RPCError<StorageRaftTypeConfig, InstallSnapshotError<StorageNodeId>>>
    {
        self.owner.send_rpc(self.target, self.target_node.as_ref(), "raft-snapshot", req).await
    }

    async fn send_vote(
        &mut self,
        req: VoteRequest<StorageNodeId>,
    ) -> Result<VoteResponse<StorageNodeId>, RPCError<StorageRaftTypeConfig, VoteError<StorageNodeId>>> {
        self.owner.send_rpc(self.target, self.target_node.as_ref(), "raft-vote", req).await
    }
}

pub async fn init_httpserver() -> std::io::Result<()> {
    // Create a configuration for the raft instance.
    let config = Arc::new(Config::default().validate().unwrap());
    /*

    let db: sled::Db = sled::open("try_my_db").unwrap();
    let t = db.open_tree("trytry").unwrap();
    let t2 = db.open_tree("trytry2").unwrap();
    let res = StorageNodeFileStore {
        last_purged_log_id: Default::default(),
        log: t,
        state_machine: Default::default(),
        vote: Default::default(),
        snapshot_idx: Arc::new(Mutex::new(0)),
        current_snapshot: Default::default()
    };

     */

    let res = StorageNodeFileStore::open_create(None, Some(()));

    let store = Arc::new(res);
    // Create a instance of where the Raft data will be stored.

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = ExampleNetwork {};

    // Create a local raft instance.
    let raft = Raft::new(ARGS.node_id, config.clone(), network, store.clone());

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.
    let app = Data::new(StorageNode {
        id: ARGS.node_id,
        addr: ARGS.addr.clone(),
        raft,
        store,
        config,
    });
    HttpServer::new(move || {
        App::new()
            .app_data(app.clone())
            // raft internal RPC
            .service(raft::append)
            .service(raft::snapshot)
            .service(raft::vote)
            // admin API
            .service(management::init)
            .service(management::add_learner)
            .service(management::change_membership)
            .service(management::metrics)
            // application API
            .service(slice::get_slice)
            .service(slice::put_slice)
    })
        .bind((ARGS.addr.to_string(), ARGS.port))?
        .run()
        .await
}

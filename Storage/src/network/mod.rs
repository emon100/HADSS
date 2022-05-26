use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use actix_web::web::Data;
use async_trait::async_trait;
use openraft::{Config, Node, Raft};
use openraft::error::{AppendEntriesError};
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

pub struct StorageNodeNetwork {}

use reqwest;
use tokio::time;
use std::future::Future;
use serde_json::json;
use tokio::time::Duration;

fn set_interval<F, Fut>(mut f: F, dur: Duration)
where
    F: Send + 'static + FnMut() -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    // Create stream of intervals.
    let mut interval = time::interval(dur);

    tokio::spawn(async move {
        // Skip the first tick at 0ms.
        interval.tick().await;
        loop {
            // Wait until next tick.
            interval.tick().await;
            // Spawn a task for this tick.
            tokio::spawn(f());
        }
    });
}

impl StorageNodeNetwork {
    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: StorageNodeId,
        target_node: Option<&Node>,
        uri: &str,
        req: Req,
    ) -> Result<Resp, RPCError<StorageNodeId, Err>>
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
impl RaftNetworkFactory<StorageRaftTypeConfig> for StorageNodeNetwork {
    type Network = StorageNodeNetworkConnection;

    async fn connect(&mut self, target: StorageNodeId, node: Option<&Node>) -> Self::Network {
        StorageNodeNetworkConnection {
            owner: StorageNodeNetwork {},
            target,
            target_node: node.cloned(),
        }
    }
}

pub struct StorageNodeNetworkConnection {
    owner: StorageNodeNetwork,
    target: StorageNodeId,
    target_node: Option<Node>,
}

#[async_trait]
impl RaftNetwork<StorageRaftTypeConfig> for StorageNodeNetworkConnection {
    async fn send_append_entries(
        &mut self,
        req: AppendEntriesRequest<StorageRaftTypeConfig>,
    ) -> Result<AppendEntriesResponse<StorageNodeId>, RPCError<StorageNodeId, AppendEntriesError<StorageNodeId>>>
    {
        self.owner.send_rpc(self.target, self.target_node.as_ref(), "raft-append", req).await
    }

    async fn send_install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<StorageRaftTypeConfig>,
    ) -> Result<InstallSnapshotResponse<StorageNodeId>, RPCError<StorageNodeId, InstallSnapshotError<StorageNodeId>>>
    {
        self.owner.send_rpc(self.target, self.target_node.as_ref(), "raft-snapshot", req).await
    }

    async fn send_vote(
        &mut self,
        req: VoteRequest<StorageNodeId>,
    ) -> Result<VoteResponse<StorageNodeId>, RPCError<StorageNodeId, VoteError<StorageNodeId>>> {
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
    let network = StorageNodeNetwork {};

    // Create a local raft instance.
    let raft = Raft::new(ARGS.node_id, config.clone(), network, store.clone());

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.
    let app = Data::new(StorageNode {
        id: ARGS.node_id,
        addr: ARGS.node_addr.clone(),
        raft,
        store,
        config,
    });


    let app1 = app.clone();

    set_interval(move || {
        let metrics = app1.raft.metrics().borrow().clone();
        let now_state: String = if metrics.current_term == 0 { "ready".into() } else { "serving".into() };
        let now_role = metrics.state.clone();


        async move {
            let json_body = json!({
  "Status": now_state,
  "NodeId": ARGS.node_id.to_string(),
  "Role": now_role,
  "Addr": ARGS.node_addr,
  "Group": "-1",
  "NodemapVersion": 1
});
            let client = reqwest::Client::new();
            client.post(format!("http://{}/heartbeat", ARGS.monitor_addr))
                  .body(json_body.to_string()).send().await;
        }
    }, Duration::new(5, 0) );
    HttpServer::new(move || {
        App::new()
            .app_data(app.clone())
            .app_data(web::PayloadConfig::new(ARGS.payload_size))
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
        .bind((ARGS.listen_addr.clone(), ARGS.port))?
        .run()
        .await
}

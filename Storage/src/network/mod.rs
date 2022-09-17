use std::future::Future;
use std::sync::Arc;

use actix_web::{App, HttpServer, web};
use actix_web::web::Data;
use async_trait::async_trait;
use openraft::{Config, Raft, RaftMetrics, RaftNetwork};
//use openraft::error::NetworkError;
//use openraft::error::RemoteError;
//use openraft::error::RPCError;
use openraft::BasicNode;
use openraft::error::{AppendEntriesError, InstallSnapshotError, VoteError};
use openraft::raft::{AppendEntriesRequest, AppendEntriesResponse};
use openraft::raft::{InstallSnapshotRequest, InstallSnapshotResponse};
use openraft::raft::{VoteRequest, VoteResponse};
use reqwest;
//use openraft::RaftNetworkFactory;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use tokio::time;
use tokio::time::Duration;

use crate::{ARGS, StorageNodeFileStore, StorageNodeId, StorageNodeRaft};
use crate::app::StorageApp;
use crate::network::raft_network_impl::StorageNodeNetwork;

//use crate::StorageRaftTypeConfig;

pub mod slice;
pub mod raft;
pub mod management;
pub mod raft_network_impl;


fn set_interval<F, Fut>(mut f: F, dur: Duration)
where
    F: Send + 'static + FnMut() -> Fut,
    Fut: Future<Output=()> + Send + 'static,
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


pub async fn init_httpserver() -> std::io::Result<()> {
    // Create a configuration for the raft instance.
    let config = Config {
        heartbeat_interval: 250,
        election_timeout_min: 299,
        ..Default::default()
    };
    let config = Arc::new(config.validate().unwrap());
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

    let store = Arc::new(StorageNodeFileStore::open_create(None, Some(())));

    // Create the network layer that will connect and communicate the raft instances and
    // will be used in conjunction with the store created above.
    let network = StorageNodeNetwork {};

    // Create a local raft instance.
    let raft: StorageNodeRaft = Raft::new(ARGS.node_id, config.clone(), network, store.clone());

    // Create an application that will store all the instances created above, this will
    // be later used on the actix-web services.
    let app = Data::new(StorageApp {
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
    }, Duration::new(5, 0));
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

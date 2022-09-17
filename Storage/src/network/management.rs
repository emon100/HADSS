use std::collections::BTreeMap;
use std::collections::BTreeSet;

use actix_web::{get, HttpResponse};
use actix_web::post;
use actix_web::Responder;
use actix_web::web;
use actix_web::web::Data;
use openraft::error::Infallible;
use openraft::Node;
use openraft::RaftMetrics;
use web::Json;

use crate::{ARGS, StorageNode, StorageNodeId, StorageNodeRequest};
use crate::app::StorageApp;
use crate::StorageRaftTypeConfig;

// --- Cluster management

/// Add a node as **Learner**.
///
/// A Learner receives log replication from the leader but does not vote.
/// This should be done before adding a node as a member into the cluster
/// (by calling `change-membership`)
#[post("/add-learner")]
pub async fn add_learner(
    app: Data<StorageApp>,
    req: Json<(StorageNodeId, String)>,
) -> actix_web::Result<impl Responder> {
    let node_id = req.0.0;
    let node = StorageNode {
        rpc_addr: req.0.1.clone(),
        api_addr: req.0.1.clone(),
    };
    let res = app.raft.add_learner(node_id, node, true).await;
    Ok(Json(res))
}

/// Changes specified learners to members, or remove members.
#[post("/change-membership")]
pub async fn change_membership(
    app: Data<StorageApp>,
    req: Json<BTreeSet<StorageNodeId>>,
) -> actix_web::Result<impl Responder> {
    let res = app.raft.change_membership(req.0, true, false).await;
    Ok(Json(res))
}

/// Initialize a single-node cluster.
#[post("/init")]
pub async fn init(app: Data<StorageApp>) -> actix_web::Result<impl Responder> {
    let mut nodes = BTreeMap::new();
    nodes.insert(app.id, StorageNode {
        rpc_addr: app.addr.clone(),
        api_addr: app.addr.clone(),
    });
    let res = app.raft.initialize(nodes).await;
    Ok(Json(res))
}

/*
/// Initialize a single-node cluster.
#[post("/nodemap")]
pub async fn nodemap(app: Data<StorageNode>, body: web::Bytes) -> impl Responder {
    let data: serde_json::Value = serde_json::from_slice(&*body.to_vec()).unwrap();
    for i in 0.. {
        if data["NodesRanged"][i] == json!(null) {
           break
        }
        if data["NodesRanged"][i]["NodesAddrs"][0].to_string().contains(&ARGS.node_addr) {
            let request = ClientWriteRequest::new(EntryPayload::Normal(StorageNodeRequest::ChangeNodeMap {}));

            let res = app.raft.initialize(nodes).await;
            return HttpResponse::Ok().body("Changed membership")
        }
    }
    HttpResponse::Ok().body("I'm healthy.")
}

 */
/// Get the latest metrics of the cluster
#[get("/metrics")]
pub async fn metrics(app: Data<StorageApp>) -> actix_web::Result<impl Responder> {
    let metrics = app.raft.metrics().borrow().clone();

    let res: Result<RaftMetrics<StorageNodeId, StorageNode>, Infallible> = Ok(metrics);
    Ok(Json(res))
}

#[get("/health")]
async fn get_health() -> impl Responder {
    HttpResponse::Ok().body("I'm healthy.")
}

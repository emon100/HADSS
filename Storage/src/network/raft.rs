use actix_web::post;
use actix_web::Responder;
use actix_web::web;
use actix_web::web::Data;
use openraft::raft::AppendEntriesRequest;
use openraft::raft::InstallSnapshotRequest;
use openraft::raft::VoteRequest;
use web::Json;

use crate::app::StorageNode;
use crate::StorageNodeId;
use crate::StorageRaftTypeConfig;

// --- Raft communication

#[post("/raft-vote")]
pub async fn vote(app: Data<StorageNode>, req: Json<VoteRequest<StorageNodeId>>) -> actix_web::Result<impl Responder> {
    let res = app.raft.vote(req.0).await;
    Ok(Json(res))
}

#[post("/raft-append")]
pub async fn append(
    app: Data<StorageNode>,
    req: Json<AppendEntriesRequest<StorageRaftTypeConfig>>,
) -> actix_web::Result<impl Responder> {
    let res = app.raft.append_entries(req.0).await;
    Ok(Json(res))
}

#[post("/raft-snapshot")]
pub async fn snapshot(
    app: Data<StorageNode>,
    req: Json<InstallSnapshotRequest<StorageRaftTypeConfig>>,
) -> actix_web::Result<impl Responder> {
    let res = app.raft.install_snapshot(req.0).await;
    Ok(Json(res))
}

use async_trait::async_trait;
use openraft::{RaftNetwork, RaftNetworkFactory};
use openraft::error::{AppendEntriesError, InstallSnapshotError, NetworkError, RemoteError, RPCError, VoteError};
use openraft::raft::{AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest, InstallSnapshotResponse, VoteRequest, VoteResponse};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::{StorageNode, StorageNodeId, StorageRaftTypeConfig};

pub struct StorageNodeNetwork {}

impl StorageNodeNetwork {
    pub async fn send_rpc<Req, Resp, Err>(
        &self,
        target: StorageNodeId,
        target_node: StorageNode,
        uri: &str,
        req: Req,
    ) -> Result<Resp, RPCError<StorageNodeId, StorageNode, Err>>
    where
        Req: Serialize,
        Err: std::error::Error + DeserializeOwned,
        Resp: DeserializeOwned,
    {
        let addr = target_node.api_addr.clone();

        let url = format!("http://{}/{}", addr, uri);
        let client = reqwest::Client::new();

        let resp = client.post(url).json(&req).send().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        let res: Result<Resp, Err> = resp.json().await.map_err(|e| RPCError::Network(NetworkError::new(&e)))?;

        res.map_err(|e| RPCError::RemoteError(RemoteError::new(target, e)))
    }
}
#[async_trait]
impl RaftNetworkFactory<StorageRaftTypeConfig> for StorageNodeNetwork{
    type Network = StorageNodeNetworkConnection;
    type ConnectionError = NetworkError;

    async fn new_client(&mut self, target_id: StorageNodeId, target_node: &StorageNode) -> Result<Self::Network, Self::ConnectionError> {
        Ok(StorageNodeNetworkConnection{
            owner: StorageNodeNetwork{},
            target_id,
            target: target_node.clone(),
        })
    }
}
pub struct StorageNodeNetworkConnection {
    owner: StorageNodeNetwork,
    target_id: StorageNodeId,
    target: StorageNode,
}

#[async_trait]
impl RaftNetwork<StorageRaftTypeConfig> for StorageNodeNetworkConnection {
    async fn send_append_entries(
        &mut self,
        req: AppendEntriesRequest<StorageRaftTypeConfig>,
    ) -> Result<AppendEntriesResponse<StorageNodeId>, RPCError<StorageNodeId, StorageNode, AppendEntriesError<StorageNodeId>>>
    {
        self.owner.send_rpc(self.target_id, self.target.clone(), "raft-append", req).await
    }

    async fn send_install_snapshot(
        &mut self,
        req: InstallSnapshotRequest<StorageRaftTypeConfig>,
    ) -> Result<InstallSnapshotResponse<StorageNodeId>, RPCError<StorageNodeId, StorageNode, InstallSnapshotError<StorageNodeId>>>
    {
        self.owner.send_rpc(self.target_id, self.target.clone(), "raft-snapshot", req).await
    }

    async fn send_vote(
        &mut self,
        req: VoteRequest<StorageNodeId>,
    ) -> Result<VoteResponse<StorageNodeId>, RPCError<StorageNodeId, StorageNode, VoteError<StorageNodeId>>> {
        self.owner.send_rpc(self.target_id, self.target.clone(), "raft-vote", req).await
    }
}

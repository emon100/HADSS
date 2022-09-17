use std::fmt::Display;
use std::sync::Arc;

use openraft::{Config, Raft};

use crate::{StorageNodeId, StorageNodeNetwork, StorageNodeRaft, StorageNodeRequest, StorageNodeResponse};
use crate::store::StorageNodeFileStore;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
pub struct StorageApp {
    pub id: StorageNodeId,
    pub addr: String,
    pub raft: StorageNodeRaft,
    pub store: Arc<StorageNodeFileStore>,
    pub config: Arc<Config>,
}

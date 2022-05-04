use std::sync::Arc;

use openraft::Config;

use crate::StorageNodeId;
use crate::ExampleRaft;
use crate::store::StorageNodeFileStore;

// Representation of an application state. This struct can be shared around to share
// instances of raft, store and more.
pub struct StorageNode {
    pub id: StorageNodeId,
    pub addr: String,
    pub raft: ExampleRaft,
    pub store: Arc<StorageNodeFileStore>,
    pub config: Arc<Config>,
}

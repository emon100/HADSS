use std::sync::Arc;
use openraft::StorageError;
use openraft::testing::Suite;
use crate::{StorageNodeFileStore, StorageNodeId};

pub async fn new_async() -> Arc<StorageNodeFileStore> {
    let res = StorageNodeFileStore::open_create(None, Some(()));

    Arc::new(res)
}

#[test]
pub fn test_mem_store() -> Result<(), StorageError<StorageNodeId>> {
    Suite::test_all(new_async)?;
    Ok(())
}

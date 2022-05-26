use std::fmt::Debug;
use std::io::Cursor;
use std::ops::{Bound, RangeBounds};
use std::sync::Arc;
use std::sync::Mutex;

use openraft::{AnyError};
use openraft::async_trait::async_trait;
use openraft::EffectiveMembership;
use openraft::Entry;
use openraft::EntryPayload;
use openraft::ErrorSubject;
use openraft::ErrorVerb;
use openraft::LogId;
use openraft::RaftLogReader;
use openraft::RaftSnapshotBuilder;
use openraft::RaftStorage;
use openraft::SnapshotMeta;
use openraft::StateMachineChanges;
use openraft::storage::LogState;
use openraft::storage::Snapshot;
use openraft::StorageError;
use openraft::StorageIOError;
use openraft::Vote;
use serde::{Deserialize};
use serde::Serialize;
use sled::{Db, IVec};
use tokio::sync::RwLock;

use crate::{ARGS, StorageNodeId};
use crate::StorageRaftTypeConfig;

pub mod fs_io;

//TODO: try delete all unwraps

/**
 * Here you will set the types of request that will interact with the raft nodes.
 * For example the `Set` will be used to write data (key and value) to the raft database.
 * The `AddNode` will append a new node to the current existing shared list of nodes.
 * You will want to add any request that can write data in all nodes here.
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum StorageNodeRequest {
    StoreData { id: String, value: Vec<u8> },
}

/**
 * Here you will defined what type of answer you expect from reading the data of a node.
 * In this example it will return a optional value from a given key in
 * the `ExampleRequest.Set`.
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StorageNodeResponse {
    pub value: Option<Vec<u8>>,
}

#[derive(Debug)]
pub struct StorageNodeStoreSnapshot {
    pub meta: SnapshotMeta<StorageNodeId>,

    /// The data of the state machine at the time of this snapshot.
    pub data: Vec<u8>,
}

/**
 * Here defines a state machine of the raft, this state represents a copy of the data
 * between each node. Note that we are using `serde` to serialize the `data`, which has
 * a implementation to be serialized. Note that for this test we set both the key and
 * value as String, but you could set any type of value that has the serialization impl.
 */
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct StorageNodeStoreStateMachine {
    pub last_applied_log: Option<LogId<StorageNodeId>>,

    // TODO: it should not be Option.
    pub last_membership: EffectiveMembership<StorageNodeId>,


    /// Application data.
    pub data: Vec<String>,
}

#[derive(Debug)]
pub struct StorageNodeFileStore {
    // pub last_purged_log_id: RwLock<Option<LogId<StorageNodeId>>>,

    /// The sled db for log and raft_state.
    /// state machine is stored in another sled db since it contains user data and needs to be export/import as a whole.
    /// This db is also used to generate a locally unique id.
    /// Currently the id is used to create a unique snapshot id.

    //In raft, only three things have to be persisted: logs, current_term, voted_for.

    /// The Raft log.
    pub log: sled::Tree,//RwLock<BTreeMap<u64, Entry<StorageRaftTypeConfig>>>,
    pub meta: sled::Tree, //voted_for and current_term

    /// The Raft state machine.
    pub state_machine: RwLock<StorageNodeStoreStateMachine>,

    /// The current granted vote.
    //pub voted_for: RwLock<Option<Vote<StorageNodeId>>>,// alternatived by meta

    pub snapshot_idx: Arc<Mutex<u64>>, //TODO: check this cache

    pub current_snapshot: RwLock<Option<StorageNodeStoreSnapshot>>,//TODO: check should cache or not
}

fn get_sled_db() -> Db {
    sled::open(format!("{}/{}",ARGS.storage_location, "database")).unwrap()
}


impl StorageNodeFileStore {
    pub fn open_create(
        open: Option<()>,
        create: Option<()>,
    ) -> StorageNodeFileStore {
        tracing::info!("open: {:?}, create: {:?}", open, create);

        let db = get_sled_db();


        let log = db.open_tree(format!("trylog")).unwrap();
        let meta = db.open_tree(format!("trymeta")).unwrap();

        let current_snapshot = RwLock::new(None);

        StorageNodeFileStore {
            //last_purged_log_id: Default::default(),
            //id: raft_state_id,
            log,
            meta,
            state_machine: Default::default(),
            //voted_for: Default::default(),
            snapshot_idx: Arc::new(Mutex::new(0)),
            current_snapshot,
        }
    }

    fn get_last_purged_log_id(&self) -> Option<LogId<StorageNodeId>>{
        match self.meta.get(b"last-purged").unwrap() {
            None => None,
            Some(res) => serde_json::from_slice::<Option<LogId<StorageNodeId>>>(&*res).unwrap()
        }
    }

    fn set_last_purged_log_id(&self, id: &LogId<StorageNodeId>) {
        self.meta.insert(b"last-purged", IVec::from(serde_json::to_vec(id).unwrap()));
    }
}

#[async_trait]
impl RaftLogReader<StorageRaftTypeConfig> for Arc<StorageNodeFileStore> {
    async fn get_log_state(&mut self) -> Result<LogState<StorageRaftTypeConfig>, StorageError<StorageNodeId>> {
        let log = &self.log;
        let last = log.iter()
                      .rev()
                      .next()
                      .map(|res| res.unwrap()).map(|(_, val)|
            serde_json::from_slice::<Entry<StorageRaftTypeConfig>>(&*val).unwrap().log_id);

        let last_purged = self.get_last_purged_log_id();

        let last = match last {
            None => last_purged,
            Some(x) => Some(x),
        };

        Ok(LogState {
            last_purged_log_id: last_purged,
            last_log_id: last,
        })
    }


    async fn try_get_log_entries<RB: RangeBounds<u64> + Clone + Debug + Send + Sync>(
        &mut self,
        range: RB,
    ) -> Result<Vec<Entry<StorageRaftTypeConfig>>, StorageError<StorageNodeId>> {
        let log = &self.log;
        let response = log.range(transform_range_bound(range))
                          .map(|res| res.unwrap())
                          .map(|(_, val)|
                              serde_json::from_slice::<Entry<StorageRaftTypeConfig>>(&*val).unwrap())
                          .collect();

        Ok(response)
    }
}

fn transform_range_bound<RB: RangeBounds<u64> + Clone + Debug + Send + Sync>(range: RB) -> (Bound<IVec>, Bound<IVec>) {
    (serialize_bound(&range.start_bound()), serialize_bound(&range.end_bound()))
}


fn serialize_bound(
    v: &Bound<&u64>,
) -> Bound<IVec> {
    match v {
        Bound::Included(v) => Bound::Included(IVec::from(&v.to_be_bytes())),
        Bound::Excluded(v) => Bound::Excluded(IVec::from(&v.to_be_bytes())),
        Bound::Unbounded => Bound::Unbounded
    }
}

#[async_trait]
impl RaftSnapshotBuilder<StorageRaftTypeConfig, Cursor<Vec<u8>>> for Arc<StorageNodeFileStore> {
    #[tracing::instrument(level = "trace", skip(self))]
    async fn build_snapshot(
        &mut self,
    ) -> Result<Snapshot<StorageNodeId, Cursor<Vec<u8>>>, StorageError<StorageNodeId>> {
        let (data, last_applied_log, last_membership);

        {
            // Serialize the data of the state machine.
            let state_machine = self.state_machine.read().await;
            data = serde_json::to_vec(&*state_machine)
                .map_err(|e| StorageIOError::new(ErrorSubject::StateMachine, ErrorVerb::Read, AnyError::new(&e)))?;

            last_applied_log = state_machine.last_applied_log;
            last_membership = state_machine.last_membership.clone();
        }

        let last_applied_log = match last_applied_log {
            None => {
                panic!("can not compact empty state machine");
            }
            Some(x) => x,
        };

        let snapshot_idx = {
            let mut l = self.snapshot_idx.lock().unwrap();
            *l += 1;
            *l
        };

        let snapshot_id = format!(
            "{}-{}-{}",
            last_applied_log.leader_id, last_applied_log.index, snapshot_idx
        );

        let meta = SnapshotMeta {
            last_log_id: last_applied_log,
            last_membership ,
            snapshot_id,
        };

        let snapshot = StorageNodeStoreSnapshot {
            meta: meta.clone(),
            data: data.clone(),
        };

        {
            let mut current_snapshot = self.current_snapshot.write().await;
            *current_snapshot = Some(snapshot);
        }

        Ok(Snapshot {
            meta,
            snapshot: Box::new(Cursor::new(data)),
        })
    }
}

const METAVOTE: &'static [u8; 9] = b"meta-vote";

#[async_trait]
impl RaftStorage<StorageRaftTypeConfig> for Arc<StorageNodeFileStore> {
    type SnapshotData = Cursor<Vec<u8>>;
    type LogReader = Self;
    type SnapshotBuilder = Self;

    #[tracing::instrument(level = "trace", skip(self))]
    async fn save_vote(&mut self, vote: &Vote<StorageNodeId>) -> Result<(), StorageError<StorageNodeId>> {
        let value = IVec::from(serde_json::to_vec(vote).unwrap());
        self.meta.insert(METAVOTE, value);
        Ok(())
    }

    async fn read_vote(&mut self) -> Result<Option<Vote<StorageNodeId>>, StorageError<StorageNodeId>> {
        match self.meta.get(METAVOTE).unwrap() {
            Some(res) => {
                Ok(Some(serde_json::from_slice::<Vote<StorageNodeId>>(&*res).unwrap()))
            }
            None => Ok(None)
        }
    }

    async fn get_log_reader(&mut self) -> Self::LogReader {
        self.clone()
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn append_to_log(
        &mut self,
        entries: &[&Entry<StorageRaftTypeConfig>],
    ) -> Result<(), StorageError<StorageNodeId>> {
        let log = &self.log;
        for entry in entries {
            log.insert(entry.log_id.index.to_be_bytes(), IVec::from(serde_json::to_vec(&*entry).unwrap()));
        }
        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn delete_conflict_logs_since(
        &mut self,
        log_id: LogId<StorageNodeId>,
    ) -> Result<(), StorageError<StorageNodeId>> {
        tracing::debug!("delete_log: [{:?}, +oo)", log_id);

        let log = &self.log;
        let keys = log.range(transform_range_bound(log_id.index..))
                      .map(|res| res.unwrap())
                      .map(|(k, _v)| k); //TODO Why originally used collect instead of the iter.
        for key in keys {
            log.remove(&key);
        }

        Ok(())
    }

    #[tracing::instrument(level = "debug", skip(self))]
    async fn purge_logs_upto(&mut self, log_id: LogId<StorageNodeId>) -> Result<(), StorageError<StorageNodeId>> {
        tracing::debug!("delete_log: [{:?}, +oo)", log_id);

        {
            let mut ld = self.get_last_purged_log_id();
            assert!(ld <= Some(log_id));
            ld = Some(log_id);
            self.set_last_purged_log_id(&ld.unwrap());
        }

        {
            let log = &self.log;

            let keys = log.range(transform_range_bound(..=log_id.index))
                          .map(|res| res.unwrap())
                          .map(|(k, _)| k);
            for key in keys {
                log.remove(&key);
            }
        }

        Ok(())
    }

    async fn last_applied_state(
        &mut self,
    ) -> Result<(Option<LogId<StorageNodeId>>, EffectiveMembership<StorageNodeId>), StorageError<StorageNodeId>> {
        let state_machine = self.state_machine.read().await;
        Ok((state_machine.last_applied_log, state_machine.last_membership.clone()))
    }

    #[tracing::instrument(level = "trace", skip(self, entries))]
    async fn apply_to_state_machine(
        &mut self,
        entries: &[&Entry<StorageRaftTypeConfig>],
    ) -> Result<Vec<StorageNodeResponse>, StorageError<StorageNodeId>> {
        let mut res = Vec::with_capacity(entries.len());

        let mut sm = self.state_machine.write().await;

        for entry in entries {
            tracing::debug!(%entry.log_id, "replicate to sm");

            sm.last_applied_log = Some(entry.log_id);

            match entry.payload {
                EntryPayload::Blank => res.push(StorageNodeResponse { value: None }),
                EntryPayload::Normal(ref req) => match req {
                    StorageNodeRequest::StoreData { id: key, value } => {
                        sm.data.push(key.clone());
                        if let Err(_) = fs_io::store_slice(key, value) {//TODO: return error when can't storage.
                        } else {
                            res.push(StorageNodeResponse { value: None })
                        }
                    }
                },
                EntryPayload::Membership(ref mem) => {
                    sm.last_membership = EffectiveMembership::new(Some(entry.log_id), mem.clone());
                    res.push(StorageNodeResponse { value: None })
                }
            };
        }
        Ok(res)
    }

    async fn get_snapshot_builder(&mut self) -> Self::SnapshotBuilder {
        self.clone()
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn begin_receiving_snapshot(&mut self) -> Result<Box<Self::SnapshotData>, StorageError<StorageNodeId>> {
        Ok(Box::new(Cursor::new(Vec::new())))
    }

    #[tracing::instrument(level = "trace", skip(self, snapshot))]
    async fn install_snapshot(
        &mut self,
        meta: &SnapshotMeta<StorageNodeId>,
        snapshot: Box<Self::SnapshotData>,
    ) -> Result<StateMachineChanges<StorageRaftTypeConfig>, StorageError<StorageNodeId>> {
        tracing::info!(
            { snapshot_size = snapshot.get_ref().len() },
            "decoding snapshot for installation"
        );

        let new_snapshot = StorageNodeStoreSnapshot {
            meta: meta.clone(),
            data: snapshot.into_inner(),
        };

        // Update the state machine.
        {
            let updated_state_machine: StorageNodeStoreStateMachine =
                serde_json::from_slice(&new_snapshot.data).map_err(|e| {
                    StorageIOError::new(
                        ErrorSubject::Snapshot(new_snapshot.meta.clone()),
                        ErrorVerb::Read,
                        AnyError::new(&e),
                    )
                })?;
            let mut state_machine = self.state_machine.write().await;
            *state_machine = updated_state_machine;
        }

        // Update current snapshot.
        let mut current_snapshot = self.current_snapshot.write().await;
        *current_snapshot = Some(new_snapshot);
        Ok(StateMachineChanges {
            last_applied: meta.last_log_id,
            is_snapshot: true,
        })
    }

    #[tracing::instrument(level = "trace", skip(self))]
    async fn get_current_snapshot(
        &mut self,
    ) -> Result<Option<Snapshot<StorageNodeId, Self::SnapshotData>>, StorageError<StorageNodeId>> {
        match &*self.current_snapshot.read().await {
            Some(snapshot) => {
                let data = snapshot.data.clone();
                Ok(Some(Snapshot {
                    meta: snapshot.meta.clone(),
                    snapshot: Box::new(Cursor::new(data)),
                }))
            }
            None => Ok(None),
        }
    }
}

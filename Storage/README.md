# StorageNode Overview

## Architecture
StorageNodes behave like a replicated state machine.
StorageNodes use Consensus module to maintain log consistency.
This means that all write operations are first recorded in a distributed log. An entry is only committed (and then applied to the actual file storage) after a quorum of nodes in the replication group agrees on it. This ensures that even if some nodes fail, the log remains consistent and data integrity is maintained. The specific consensus algorithm used can vary (e.g., Raft or Paxos).
Then use the log to maintain file consistency.

### Write Operation Flow

1.  **Log Entry:** When a write request (e.g., new file or update) is received, the StorageNode first proposes it as an entry to the distributed log for its replication group.
2.  **Consensus:** The consensus module ensures this log entry is replicated to a quorum of nodes in the group.
3.  **Local Application:** Once the log entry is committed, the StorageNode pulls this entry from its local log.
4.  **Data Fetch (if applicable):** For operations like "copy-on-write" or partial updates, the node might fetch existing data.
5.  **Store Data:** The file content is written to the node's local storage (e.g., a folder or a block device). The metadata (filename, version, etc.) associated with this content is also updated based on the log entry.

### Read Operation Flow

1.  **Check Log (Consistency):** To ensure up-to-date reads (depending on the consistency policy specified by the client), the StorageNode may first consult its (and potentially other nodes') log to ensure there are no pending committed writes for the requested data.
2.  **Verify Local Version:** The node checks if its local storage has the correct version of the file/slice as indicated by its log.
3.  **Read Data:** If the local version is current, the node reads the data from its local storage and returns it.
>   Note: The `NodeStatusVersion` mentioned in the main project README is a planned enhancement for more granular status tracking and is yet to be fully implemented in this layer.

## HTTP endpoints
### /health
This endpoint shows health information of the Storage node.

### /slice/:id
* GET <id> ;return the file by its id
* HEAD <id> ; return metadata of the file by its id
* DELETE <id>+ ;return if the operation is successful

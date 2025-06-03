# HADSS
HADSS is short for High Availability Distributed Storage System.

HADSS can be used as backend of a Object Storage Service.

## Design Philosophy
HADSS is designed with high availability as a core principle:
*   **No Single Point of Failure:** The system is architected to tolerate failures of individual components.
*   **Responsive Scaling:** HADSS remains responsive even when the cluster scales up or down.

HADSS can answer basic requests even when the entire Index Layer is down.

## Architecture
HADSS consists of 3 layers:
- Gateway Layer
- Index Layer
- Storage Layer

Each Layer contain nodes in different regions, data centers or racks.

### Gateway Layer
Gateway Layer faces users' requests.

Gateway Layer wraps different kinds of storage services into RPCs to the Storage Layer and Index Layer.
e.g. The business logic for an Object Storage Service is usually on Gateway Layer.

The best practice in production is to place **Load Balancers** before Gateway Nodes.

### Index Layer
Index Layer monitors Storage Layer status and performs recovery. e.g.
- Monitor Storage Node cluster scaling (up or down)
- Monitor health of Storage Nodes
- Provide Storage Nodes' information to other layers
- Detect and recover data corruptions (Hard disk sometimes corrupts files)
- Detect and recover Node Failures
In the current implementation, the `Monitor` component handles the primary responsibilities of the Index Layer.

### Storage Layer
Storage Layer stores. It receives (Handler, ConsistencyPolicy, NodeStatusVersion (to be implemented)]) to store a file.
It can auto-balancing the storage usage between storage nodes by node status.

## How it works
The System's sequence diagram when storing a file by Object Storage Gateway.
```mermaid
sequenceDiagram
    autonumber
    actor User
    participant GatewayNode
    participant StorageNode
    participant StorageNode..N
    
    Note right of User: may be behind of LoadBalancers
    User->>GatewayNode: Sending a file.
    activate GatewayNode
    
    loop sometimes
        GatewayNode->IndexLayer: Syncing Storage Cluster's node status, which contains information about all storage nodes.
    end
    
    GatewayNode->>GatewayNode: Calculates metadata and determines the target StorageNode(s) for the data.
    
    Note over GatewayNode, GatewayNode: GatewayNode prepares to store metadata and file.
    Note left of StorageNode: If StorageNode is not actually leader of the node group,
    Note left of StorageNode: it will forward the request to the real leader.
    GatewayNode-)StorageNode: Putting data (including metadata) asynchronously.
    activate StorageNode
    StorageNode-)StorageNode..N: Copy data
    activate StorageNode..N
    Note over StorageNode, StorageNode..N: Quorum write.
    StorageNode..N-->>StorageNode: Copy complete.
    deactivate StorageNode..N
    StorageNode-->>GatewayNode: data stored.
    deactivate StorageNode
    GatewayNode-->>User: File is stored.
    deactivate GatewayNode
```

The System's sequence diagram when fetching a file by Object Storage Gateway.
```mermaid
sequenceDiagram
    autonumber
    actor User
    
    User->>GatewayNode: Sending the file's identifier.
    Note over User, GatewayNode: LoadBalancers before GatewayNodes.
    activate GatewayNode
    
    loop sometimes
        GatewayNode->IndexLayer: Syncing Storage Cluster's state.
    end
    
    GatewayNode->>GatewayNode: Calculating metadata location.
    Note over GatewayNode, GatewayNode: This calculation uses Consistent Hashing. 
    
    GatewayNode->>GatewayNode: Calculating data stripe locations.
    
    Note over GatewayNode, GatewayNode: Use quorum read to make sure GatewayNode read the newest content. 
    par Collecting data stripes.
    GatewayNode->StorageNode0..N: Collecting stripesN as step 4 5.
    and
    GatewayNode->StorageNode0..N: Collecting stripesN as step 4 5.
    end
    GatewayNode-->>User: Returning the file.
    deactivate GatewayNode
    
```

The System's sequence diagram when clusters scale up/down.
```mermaid
sequenceDiagram
    autonumber
    participant IndexLayer
    participant StorageNode..N 
    
    StorageNode..N->IndexLayer: Heartbeat from node1, add a new StorageNode
    StorageNode..N->IndexLayer: Heartbeat from node2, add a new StorageNode
    StorageNode..N->IndexLayer: Heartbeat from node3, add a new StorageNode
    
    Note right of IndexLayer: IndexLayer collects healthy StorageNodes
    IndexLayer->>IndexLayer: Calculating new node status and data distribution.
    IndexLayer->>StorageNode..N: Ask node1,2,3 to join group1 and do data re-distribution.
    StorageNode..N-->>IndexLayer: node1,2,3 Ready
    IndexLayer->>IndexLayer: Put the new data distribution into uncommitted version of node status.
    
    StorageNode..N->IndexLayer: Heartbeat from node1, leader of group 1, almost full, need data re-distribution
    StorageNode..N->IndexLayer: Heartbeat from node4, add a new StorageNode
    StorageNode..N->IndexLayer: Heartbeat from node5, add a new StorageNode
    StorageNode..N->IndexLayer: Heartbeat from node6, add a new StorageNode
    
    Note right of IndexLayer: IndexLayer collects healthy StorageNodes
    IndexLayer->>IndexLayer: Calculating new node status and data distribution.
    IndexLayer->>StorageNode..N: Ask group1 to shrink data range and node4,5,6 to join group2 and extend data range.
    StorageNode..N-->>IndexLayer: node1,2,3,4,5,6 Ready
    IndexLayer->>IndexLayer: Put the new data distribution into uncommitted version of node status.
    
    loop
        IndexLayer->>IndexLayer: When the last stable node status expires, replace it with uncommitted version.
    end
```

**Data Distribution and Re-balancing:**
The scaling of Storage Node clusters (adding or removing nodes) itself is less critical than the subsequent data re-balancing. If adding or removing nodes doesn't trigger a change in data distribution, the immediate impact is minimal. However, data re-balancing, which redistributes data across the available nodes, is a crucial process that directly affects data placement and availability.

## Testing
### E2E test
Execute `run.sh` to test run the whole system.

### Integration test
StorageConnector has Integration tests in `connector_test.go` to verify the interface implementation correctness.

### Unit test
StorageConnector has unit tests in `connector_test.go` to verify the right behaviour.

## Future Work

HADSS is an evolving system. Here are some areas planned for future development:

*   **NodeStatusVersion Implementation:** Complete the implementation of `NodeStatusVersion` in the Storage Layer to provide more granular control and consistency.
*   **Advanced Data Integrity:** Enhance mechanisms for detecting and recovering from data corruption, potentially including checksum validation and automated repair processes.
*   **Enhanced Index Layer Capabilities:** Further develop the Index Layer/Monitor component to provide more sophisticated cluster management and recovery features.
*   **Security Enhancements:** Implement robust authentication, authorization, and data encryption mechanisms.
*   **Broader Protocol Support:** Extend Gateway Layer to support a wider range of storage protocols.

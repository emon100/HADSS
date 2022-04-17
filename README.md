# HADSS
HADSS is short for High Availability Distributed Storage System.

HADSS can be used as backend of a Object Storage Service.

## Design Philosophy
High Availability in HADSS stands for:
1. No Single Point Failure.
2. Responsive even when clusters scale up/down.

HADSS can answer basic requests even when the entire Monitor Layer is down.

## Architecture
HADSS consists of 3 layers:
- Gateway Layer
- Monitor Layer
- Storage Layer

Each Layer contain nodes in different regions, data centers or racks.

### Gateway Layer
Gateway Layer faces users' requests.

Gateway Layer warps different kinds of storage services into RPCs to the Storage Layer and Monitor Layer.
e.g. The business logic for an Object Storage Service is usually on Gateway Layer.

The best practice in production is to place **Load Balancers** before Gateway Nodes.

### Monitor Layer
Monitor Layer monitors Storage Layer status and performs recovery. e.g.
- Monitor health of Storage Nodes
- Monitor Storage Nodes cluster scales up/down
- Detect and recover data corruptions (Hard disk sometimes corrupts files)
- Detect and recover Node Failures (Hard disk sometimes corrupts files)

### Storage Layer
Storage Layer stores. It receives (Handler, ConsistencyPolicy, NodeStatusVersion(to be implemented)]) to store a file.
It can auto-balancing the storage usage between storage nodes by NodeStatus.

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
        GatewayNode->MonitorCluster: Syncing StorageClusters' nodestatus which contains all storage nodes infomation.
    end
    
    GatewayNode->>GatewayNode: Calculates metadata and determines StorageNode used to store data.
    
    Note over GatewayNode, GatewayNode: GatewayNode storing metadata and file.
    Note left of StorageNode: If StorageNode is not actually leader of the node group,
    Note left of StorageNode: it will forward the request to the real leader.
    GatewayNode-)StorageNode: Putting data(metadata) async.
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
        GatewayNode->MonitorCluster: Syncing StorageCluster's states.
    end
    
    GatewayNode->>GatewayNode: Calculating metadata's position. 
    Note over GatewayNode, GatewayNode: This calculation uses Consistent Hashing. 
    
    GatewayNode->>GatewayNode: Calculating all data stripes' position. 
    
    Note over GatewayNode, GatewayNode: Use quorum read to make sure GatewayNode read the newest content. 
    par Collecting data stripes.
    GatewayNode->StorageNode0..N: Collecting stripesN as step 4 5.
    and
    GatewayNode->StorageNode0..N: Collecting stripesN as step 4 5.
    end
    GatewayNode-->>User: Returing the file.
    deactivate GatewayNode
    
```

The System's sequence diagram when clusters scale up/down.
```mermaid
sequenceDiagram
    autonumber
    participant MonitorCluster
    participant StorageNode..N 
    
    StorageNode..N->MonitorCluster: Heartbeat from node1, add a new StorageNode
    StorageNode..N->MonitorCluster: Heartbeat from node2, add a new StorageNode
    StorageNode..N->MonitorCluster: Heartbeat from node3, add a new StorageNode
    
    Note right of MonitorCluster: MonitorCluster collects healthy StorageNodes
    MonitorCluster->>MonitorCluster: Calculating new nodestatus and data distribution.
    MonitorCluster->>StorageNode..N: Ask node1,2,3 to join group1 and do data re-distribution.
    StorageNode..N-->>MonitorCluster: node1,2,3 Ready
    MonitorCluster->>MonitorCluster: Put the new data distribution into uncommited version of node status.
    
    StorageNode..N->MonitorCluster: Heartbeat from node1, leader of group 1, almost full, need data re-distribution
    StorageNode..N->MonitorCluster: Heartbeat from node4, add a new StorageNode
    StorageNode..N->MonitorCluster: Heartbeat from node5, add a new StorageNode
    StorageNode..N->MonitorCluster: Heartbeat from node6, add a new StorageNode
    
    Note right of MonitorCluster: MonitorCluster collects healthy StorageNodes
    MonitorCluster->>MonitorCluster: Calculating new nodestatus and data distribution.
    MonitorCluster->>StorageNode..N: Ask group1 to shrink data range and node4,5,6 to join group2 and extend data range.
    StorageNode..N-->>MonitorCluster: node1,2,3,4,5,6 Ready
    MonitorCluster->>MonitorCluster: Put the new data distribution into uncommited version of nodestatus.
    
    loop
        MonitorCluster->>MonitorCluster: When the last stable nodestatus expires, replace it with uncommitted version.
    end
```


Storage Node Clusters scale up or down don't matter.
Storage Nodes' data re-balancing matters.

Why: if adding nodes or losing nodes won't change data distribution, then it doesn't matter.
Only data re-balancing matters, because it will change data distribution.

## Testing
### Unit test

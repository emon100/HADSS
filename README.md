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
    
    User->>GatewayNode: Sending a file.
    Note over User, GatewayNode: LoadBalancers before GatewayNodes.
    activate GatewayNode
    
    loop sometimes
        GatewayNode->MonitorCluster: Syncing StorageCluster's nodestatuse.
    end
    
    GatewayNode->>GatewayNode: Calculates metadata and determines StorageNode used to store data.
    
    Note over GatewayNode, GatewayNode: GatewayNode storing metadata and file.
    alt If StorageNodeLeader is healthy:
        GatewayNode-)StorageNodeLeader: Putting data(metadata) async.
        activate StorageNodeLeader
        StorageNodeLeader-)StorageNode..N: Copy data
        activate StorageNode..N
        Note over StorageNodeLeader, StorageNode..N: 
        StorageNode..N-->>StorageNodeLeader: Copy complete.
        deactivate StorageNode..N
        StorageNodeLeader-->>GatewayNode: data stored.
        deactivate StorageNodeLeader
    else else fallback to GatewayNodeY (follower)
        GatewayNode-)StorageNodeY: Putting data(metadata) async.
        activate StorageNodeY
        StorageNodeY->StorageNode..N: Find the real leader and forward request. Then same as step 4-6.
        StorageNodeY-->>GatewayNode: data stored.
        deactivate StorageNodeY
    end
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

Storage Node Clusters scale up or down don't matter.
Storage Nodes' data re-balancing matters.

Why: if adding nodes or losing nodes won't change data distribution, then it doesn't matter.
Only data re-balancing matters, because it will change data distribution.

## Testing
### Unit test

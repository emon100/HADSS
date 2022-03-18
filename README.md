# HADSS
HADSS is short for High Availability Distributed Storage System.

## Overview
HADSS can be used as backend of a Object Storage Service.

## Design
### Philosophy
High Availability in HADSS stands for:
1. No Single Point Failure.
2. Responsive even when clusters scale up/down.

HADSS can answer basic requests even when the entire Monitor Layer is down.

### Architecture
HADSS consists of 3 layers: **Gateway Layer**, **Monitor Layer**
and **Storage Layer**.

Each Layer contains nodes in different 

#### Gateway Layer
Gateway Layer faces users' requests.
The best practice is to place **Load Blancers** before Gateway Nodes.

Gateway Layer warps different kinds of storage services into RPCs to the
Storage Layer and Monitor Layer. e.g. The business logic for 
a Object Storage Service is usually on Gateway Layer.

Gateway Layer can be stateless or stateful. A stateless Gateway Layer utilze

#### Monitor Layer

#### Storage Layer

### Sequence Diagram
The System's sequence diagram when storing a file by Object Storage Gateway.
```mermaid
sequenceDiagram
    autonumber
    actor User
    
    User->>GatewayNode: Sending a file.
    Note over User, GatewayNode: LoadBalancers before GatewayNodes.
    activate GatewayNode
    
    loop sometimes
        GatewayNode->MonitorCluster: Syncing StorageCluster's states.
    end
    
    GatewayNode->>GatewayNode: Calculating metadata, Striping Data and calculating StorageNodes used to store data.
    Note over GatewayNode, GatewayNode: File stripes balance loads between StorageNodes.
    
    par GatewayNode storing metadata.
        alt If GatewayNodeX is healthy:
            GatewayNode-)StorageNodeX: Putting data(metadata) async.
            activate StorageNodeX
            StorageNodeX-)StorageNode..N: Copy data
            activate StorageNode..N
            Note over StorageNodeX, StorageNode..N: Mainly for availabilty goal.
            StorageNode..N-->>StorageNodeX: Copy complete.
            deactivate StorageNode..N
            StorageNodeX-->>GatewayNode: data stored.
            deactivate StorageNodeX
        else else fallback to GatewayNodeY(Another node can store the metadata.)
            GatewayNode-)StorageNodeY: Putting data(metadata) async.
            activate StorageNodeY
            StorageNodeY-)StorageNode..N: Copy data
            activate StorageNode..N
            Note over StorageNodeY, StorageNode..N: Mainly for availabilty goal.
            StorageNode..N-->>StorageNodeY: Copy complete.
            deactivate StorageNode..N
            StorageNodeY-->>GatewayNode: data stored.
            deactivate StorageNodeY
        end
    and GatewayNode storing data stripes.
        GatewayNode-)StorageNodeY: Putting data(data stripes) async.
        activate StorageNodeY
        StorageNodeY-)StorageNode..N: Copy data
        activate StorageNode..N
        Note over StorageNodeY, StorageNode..N: Mainly for availabilty goal.
        StorageNode..N-->>StorageNodeY: Copy complete.
        activate StorageNode..N
        StorageNodeY-->>GatewayNode: data stored.
        deactivate StorageNodeY
    end
    GatewayNode-->>User: File is stored.
    deactivate GatewayNode
    
    loop sometimes
        StorageNode..N-)StorageNode..N: Copy data between Nodes.
        Note over StorageNode..N, StorageNode..N:Mainly for consistency goal.
    end
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
    
    GatewayNode-)StorageNode0..N: Fetching data(metadata) async.
    StorageNode0..N-->>GatewayNode: Giving data(metadata) back.
    
    GatewayNode->>GatewayNode: Calculating all data stripes' position. 
    par Collecting data stripes.
    GatewayNode->StorageNode0..N: Collecting stripesN as step 4 5.
    and
    GatewayNode->StorageNode0..N: Collecting stripesN as step 4 5.
    end
    GatewayNode-->>User: Returing the file.
    deactivate GatewayNode
    
```

### Testing

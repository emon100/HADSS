# Monitor
Monitor is the brain of Index Layer.
It monitors Storage Layer status and performs recovery. e.g.
- Monitor Storage Nodes cluster scales up/down
- Monitor health of Storage Nodes
- Provide Storage Nodes' information to other layers
- Detect and recover data corruptions (Hard disk sometimes corrupts files)
- Detect and recover Node Failures

## Endpoints
- `/heartbeat` StorageNode request this Endpoint to update status and report health.
e.g.
```json5
//request:
{
  "Status": "ready",
  "NodeId": "1",
  "Addr": "127.0.0.1:12345",
  "NodemapVersion": 1,
}
```
```json5
//response:
{
  "NodemapVersion": 2, //Maybe Nodemap updated
  "groupTo": "1",
}
```
- `/nodemap?NodeId=` Get nodemap. e.g.
```json5
//response:
// map[NodeId] -> GroupId
// map[GroupId]-> range
{
  "Nodemap": {
    "1": {
      "range": ["0","7FFFFFFFFFFFF..."],
      "leader": "127.0.0.1:12345"
    },
    "2": {
      "range": ["8FFFFFFFFFF...","FFFFFFFFFFFF.."],
      "leader": "127.0.0.1:12346"
    },
    //Could have more...
  }
}
```

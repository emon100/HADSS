package main

import (
	"context"
	"encoding/json"
	"fmt"
	"github.com/gin-gonic/gin"
	clientv3 "go.etcd.io/etcd/client/v3"
	"go.etcd.io/etcd/client/v3/concurrency"
	"log"
	"time"
)

func init() {
	log.SetPrefix("Monitor: ")
}

type NodeRange struct {
	NodesAddrs []string
	Range      []string
}
type Nodemap struct {
	NodesRanges []NodeRange
}

//type Nodemap = map[string]NodeRange

type MonitorController struct {
	//TODO nodemap cache? nodeMap Nodemap
	etcdCli *clientv3.Client
	ctx     context.Context
}

//TODO
// - /heartbeat
//collect data from nodes and then store it in etcd in a lease, then response.
type StorageNodeHeartBeat struct {
	Status         string
	NodeId         string
	Role           string
	Addr           string
	Group          string
	NodemapVersion int
}

func (s *MonitorController) heartbeat(c *gin.Context) {
	var heartbeat StorageNodeHeartBeat
	decoder := json.NewDecoder(c.Request.Body)
	err := decoder.Decode(&heartbeat)
	if err != nil {
		c.AbortWithError(400, fmt.Errorf("json decode body error: %e", err))
		return
	}
	log.Printf("/heartbeat: %+v", heartbeat)
	c.PureJSON(200, s)
}

//TODO
// - /nodemap
//each monitor node try to gain a repeated distributed lock and the monitor gaining the lock should calculate a node status
//and put the result to etcd cluster. monitors which can't gain the lock read the origin result.
func (s *MonitorController) getNodemap(c *gin.Context) {
	//get all nodeid
	//split/rearrange the fullest segment
	res, err := s.etcdCli.Get(s.ctx, "/nodemap")
	if err != nil {
		c.AbortWithError(500, fmt.Errorf("get nodemap store error: %e", err))
		return
	}
	if len(res.Kvs) != 1 {
		c.AbortWithError(500, fmt.Errorf("etcd return store length incorrect: %+v, error: %e", res.Kvs, err))
		return
	}
	/*
		var nodemap Nodemap
		json.Unmarshal(res.Kvs[0].Value, &nodemap)
		if err ...

	*/
	c.Data(200, "application/json", res.Kvs[0].Value)
}

/*
//response:
// map[GroupId]-> range
{
"Nodemap": {
	"0": {
		"Range": ["0","7FFFFFFFFFFFF..."],
		"NodeAddrs": ["127.0.0.1:12345"]
	},
	"1": {
		"Range": ["8FFFFFFFFFF...","FFFFFFFFFFFF.."],
		"NodeAddrs": ["127.0.0.1:12346"]
	}, //Could have more...
	}
}

*/

/*
{
  "Status": "ready",
  "NodeId": "1",
  "Role": "Leader",
  "Addr": "127.0.0.1:12345",
  "Group": "0",
  "NodemapVersion": 1
}

func (c MonitorController)tryCalculateNewNodemap() error {
	nodes, err := c.CollectNodesInfo()
	for StorageNode {
		if status != ready or NodemapVersion < NowNodemapVersion {
			can't update nodemap now because newest nodemap haven't been applied to every nodes yet.
			releaseLock after a while using lease
			return
		}
	}
	newNodemap =	ProcessNodesInfo
	nodemapVersion++
}

*/
func (s *MonitorController) CollectNodesInfo() ([]StorageNodeHeartBeat, error) {
	gresp, err := s.etcdCli.Get(s.ctx, "/nodes", clientv3.WithPrefix())
	if err != nil {
		return nil, err
	}
	res := make([]StorageNodeHeartBeat, len(gresp.Kvs))
	for i, ev := range gresp.Kvs {
		err = json.Unmarshal(ev.Value, &res[i])
		if err != nil {
			return nil, err
		}
	}
	log.Println("CollectNodesInfo: res=%+v", res)
	return res, nil
}

/*
func MockProcessNodesInfo() {
	for i, i2 := range groupInfo {
		if groupTooFull {
			trySplitSegment
		}
		if groupNeedRepairing {
			tryAddNewNodeToGroup
		}
		return NewDistribution
	}
}

*/

//TODO: check all context.
func (s *MonitorController) startRepeatingCalculation() {
	session, err := concurrency.NewSession(s.etcdCli, concurrency.WithTTL(1)) // Default lease 60 seconds
	mutex := concurrency.NewMutex(session, "/NodeStatus")
	if err != nil {
		panic("failed create new session.")
	}
	go func() {
		defer session.Close()
		for {
			log.Printf("Try lock")
			err = mutex.Lock(context.Background())
			log.Printf("get lock")

			//do all calculation
			//nodes, err := s.CollectNodesInfo()
			//nodemap := s.tryCalculateNewNodemap(nodes)
			//putNodemap(nodemap)
			mutex.Unlock(context.Background())
			time.Sleep(10 * time.Second)
		}
	}()
}

func GetMockMonitorController(etcdAddr string) MonitorController {
	cli, err := clientv3.New(clientv3.Config{
		Endpoints:   []string{etcdAddr},
		DialTimeout: 10 * time.Second,
	})

	if err == context.DeadlineExceeded {
		panic("Etcd client initialize failed.")
	}

	controller := MonitorController{etcdCli: cli, ctx: context.Background()}
	controller.startRepeatingCalculation()

	//controller.nodeMap.NodesRanges = make([]NodeRange, 1)
	//controller.nodeMap.NodesRanges[0] = NodeRange{[]string{"http://localhost:21001", "http://localhost:21002", "http://localhost:21003"}, []string{"0", "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"}}

	return controller
}

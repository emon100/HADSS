package main

import (
	"context"
	"encoding/json"
	"fmt"
	"github.com/gin-gonic/gin"
	clientv3 "go.etcd.io/etcd/client/v3"
	"go.etcd.io/etcd/client/v3/concurrency"
	"io/ioutil"
	"log"
	"math/big"
	"net/http"
	"strings"
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
	NodesRanges    []NodeRange
	NodemapVersion int64
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
	NodemapVersion int64
}

func (s *MonitorController) heartbeat(c *gin.Context) {
	body, err := ioutil.ReadAll(c.Request.Body)
	if err != nil {
		c.AbortWithError(400, fmt.Errorf("HTTP body error: %e", err))
		return
	}

	var heartbeat StorageNodeHeartBeat
	err = json.Unmarshal(body, &heartbeat)
	if err != nil {
		c.AbortWithError(400, fmt.Errorf("json decode body error: %e", err))
		return
	}
	log.Printf("/heartbeat: %+v", heartbeat)
	s.etcdCli.Put(s.ctx, "/nodes/"+heartbeat.NodeId, string(body))
	//c.PureJSON(200, s)
	c.Status(200)
}

func (s *MonitorController) getNodemapFromEtcd() (*Nodemap, error) {
	res, err := s.etcdCli.Get(s.ctx, "/nodemap")
	if err != nil {
		return nil, err
	}
	if len(res.Kvs) != 1 {
		return nil, fmt.Errorf("nodemap length not correct")
	}
	var nodemap Nodemap
	err = json.Unmarshal(res.Kvs[0].Value, &nodemap)
	if err != nil {
		return nil, err
	}
	return &nodemap, nil
}

//TODO
// - /nodemap
//each monitor node try to gain a repeated distributed lock and the monitor gaining the lock should calculate a node status
//and put the result to etcd cluster. monitors which can't gain the lock read the origin result.
func (s *MonitorController) getNodemap(c *gin.Context) {
	//get all nodeid
	//split/rearrange the fullest segment
	res, err := s.getNodemapFromEtcd()
	if err != nil {
		c.AbortWithError(500, fmt.Errorf("get nodemap store error: %e", err))
		return
	}
	/*
		var nodemap Nodemap
		json.Unmarshal(res.Kvs[0].Value, &nodemap)
		if err ...

	*/
	c.JSON(200, res)
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

*/

func (c MonitorController) tryCalculateNewNodemap(nodemap []StorageNodeHeartBeat, oldNodemap *Nodemap) (*Nodemap, error) {
	readyNodecount := 0
	var newGroups []NodeRange
	for _, heartbeat := range nodemap {
		if heartbeat.Status == "ready" {
			readyNodecount++
			if readyNodecount%3 == 1 {
				newGroup := NodeRange{
					Range:      []string{"0", "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"},
					NodesAddrs: []string{},
				}
				newGroups = append(newGroups, newGroup)
			}
			newGroups[(readyNodecount-1)/3].NodesAddrs = append(newGroups[(readyNodecount-1)/3].NodesAddrs, heartbeat.Addr)
		}
	}

	newGroupCounts := readyNodecount / 3
	if newGroupCounts == 0 {
		return oldNodemap, nil
	}
	newGroups = newGroups[:newGroupCounts]
	rangeSplit := newGroupCounts + len(oldNodemap.NodesRanges)

	for _, newGroup := range newGroups {
		log.Printf("newGroup: %+v", newGroups)
		//TODO new err handle
		log.Printf("creating newGroup: %+v", newGroups)
		http.Post("http://"+newGroup.NodesAddrs[0]+"/init", "application/json", strings.NewReader("{}"))
		time.Sleep(1 * time.Second)
		http.Post("http://"+newGroup.NodesAddrs[0]+"/add-learner", "application/json", strings.NewReader("[2, \""+newGroup.NodesAddrs[1]+"\"]"))
		time.Sleep(1 * time.Second)
		http.Post("http://"+newGroup.NodesAddrs[0]+"/add-learner", "application/json", strings.NewReader("[3, \""+newGroup.NodesAddrs[2]+"\"]"))
		time.Sleep(1 * time.Second)
		http.Post("http://"+newGroup.NodesAddrs[0]+"/change-membership", "application/json", strings.NewReader("[1, 2, 3]"))
		log.Printf("created newGroup")
	}
	oldNodemap.NodesRanges = append(oldNodemap.NodesRanges, newGroups...)
	// Set ranges //TODO last one will not fill the whole area because 10/3*3 == 9 but 9 != 10
	i := new(big.Int)
	i.SetString("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16)
	each := i.Div(i, big.NewInt(int64(rangeSplit)))
	now := big.NewInt(0)
	for i := range oldNodemap.NodesRanges {
		oldNodemap.NodesRanges[i].Range[0] = now.Text(16)
		now.Add(now, each)
		oldNodemap.NodesRanges[i].Range[1] = now.Text(16)
	}
	oldNodemap.NodesRanges[rangeSplit-1].Range[1] = "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"
	return oldNodemap, nil
}
func (s *MonitorController) CollectNodesInfo() ([]StorageNodeHeartBeat, error) {
	gresp, err := s.etcdCli.Get(s.ctx, "/nodes", clientv3.WithPrefix())
	if err != nil || len(gresp.Kvs) == 0 {
		return nil, err
	}
	res := make([]StorageNodeHeartBeat, len(gresp.Kvs))
	for i, ev := range gresp.Kvs {
		err = json.Unmarshal(ev.Value, &res[i])
		if err != nil {
			return nil, err
		}
	}
	log.Printf("CollectNodesInfo: res=%+v", res)
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
			func() {
				log.Printf("Try lock")
				err = mutex.Lock(context.Background())
				defer time.Sleep(10 * time.Second)
				defer mutex.Unlock(context.Background())
				if err != nil {
					log.Printf("get lock error")
					return
				}
				log.Printf("get lock")

				//TODO handle errors
				nodes, err := s.CollectNodesInfo()
				if err != nil {
					log.Printf("Collect Node error")
					return
				}
				oldNodemap, err := s.getNodemapFromEtcd()
				if err != nil { //TODO delete initialize nodemap when err, just for test
					oldNodemap = new(Nodemap)
					log.Printf("GetNodemapFromEtcdError")
				}
				log.Printf("original nodemap %+v", oldNodemap)
				newNodemap, _ := s.tryCalculateNewNodemap(nodes, oldNodemap)
				json, _ := json.Marshal(newNodemap)
				log.Printf("new nodemap %+v", newNodemap)
				s.etcdCli.Put(s.ctx, "/nodemap", string(json))
			}()
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

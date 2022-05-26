package main

import (
	connector "HADSS/StorageConnector"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"github.com/gin-gonic/gin"
	"io/ioutil"
	"log"
	"math/big"
	"net/http"
)

type NodeRange struct {
	NodesAddrs []string
	Range      []string
}
type Nodemap struct {
	NodesRanges    []NodeRange
	NodemapVersion int64
}

type GatewayController struct {
	MonitorAddr string
}

func (self GatewayController) getStorageNodeAddr(hash []byte) string {
	g, err := http.Get(self.MonitorAddr + "/nodemap?raw")
	if err != nil {
		return ""
	}
	defer g.Body.Close()
	storageAddr, err := ioutil.ReadAll(g.Body)
	fmt.Println("%s", string(storageAddr))
	nodemap := Nodemap{}
	err = json.Unmarshal(storageAddr, &nodemap)
	if err != nil {
		return ""
	}

	now := big.NewInt(0)
	now.SetBytes(hash)
	nodeAddr := ""
	for _, nodeRange := range nodemap.NodesRanges {
		l := big.NewInt(0)
		r := big.NewInt(0)
		l.SetString(nodeRange.Range[0], 16)
		r.SetString(nodeRange.Range[1], 16)
		if (l.Cmp(now) == -1 || l.Cmp(now) == 0) && (now.Cmp(r) == -1 || now.Cmp(r) == 0) {
			nodeAddr = nodeRange.NodesAddrs[0]
			break
		}
	}
	log.Printf("GetStorageNodeAddr: %s", nodeAddr)
	return nodeAddr
}

func (self GatewayController) getId(c *gin.Context) {
	id := c.Param("id")
	h := sha256.Sum256([]byte(id))
	log.Printf("getId: id: %s, SHA-256: %s", id, hex.EncodeToString(h[:]))

	conn := connector.NewBasicConnection(self.getStorageNodeAddr(h[:]), connector.StrongConsistency)
	res := append(h[:], []byte(id)...)
	data, err := conn.GetSlice(res)
	if err != nil {
		c.AbortWithError(502, err)
		return
	}
	c.Data(200, "application/octet-stream", data)
}

func (self GatewayController) putId(c *gin.Context) {
	id := c.Param("id")
	h := sha256.Sum256([]byte(id))
	log.Printf("putId: id: %s, SHA-256: %s", id, hex.EncodeToString(h[:]))

	defer c.Request.Body.Close()
	contentLength := c.Request.ContentLength
	if contentLength <= 0 {
		c.AbortWithError(502, fmt.Errorf("can't support streaming request"))

	}
	data, err := ioutil.ReadAll(c.Request.Body)

	if err != nil {
		c.AbortWithError(502, fmt.Errorf("server create slice fail. Detail: %w", err))
		return
	}

	res := append(h[:], []byte(id)...)
	conn := connector.NewBasicConnection(self.getStorageNodeAddr(h[:]), connector.StrongConsistency)
	err = conn.PutSlice(res, data)
	if err != nil {
		c.AbortWithError(502, err)
		return
	}
	c.Data(200, "application/octet-stream", []byte("Upload finished."))
}

func (self GatewayController) startServer(listenAddr string) {
	r := gin.Default()
	r.GET("/id/:id", self.getId)
	r.PUT("/id/:id", self.putId)

	err := r.Run(listenAddr)
	if err != nil {
		fmt.Println(err)
	}
}

package main

import (
	connector "HADSS/StorageConnector"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"github.com/gin-gonic/gin"
	"io/ioutil"
	"log"
	"net/http"
)

type GatewayController struct {
	MonitorAddr string
}

func (self GatewayController) getStorageNodeAddr() string {
	g, err := http.Get(self.MonitorAddr + "/storageNode?raw")
	if err != nil {
		return ""
	}
	defer g.Body.Close()
	storageAddr, err := ioutil.ReadAll(g.Body)
	if err != nil {
		return ""
	}
	return string(storageAddr)
}

func (self GatewayController) getId(c *gin.Context) {
	id := c.Query("id")
	h := sha256.Sum256([]byte(id))
	log.Printf("getId: SHA-256: %s", hex.EncodeToString(h[:]))

	conn := connector.NewBasicConnection(self.getStorageNodeAddr(), connector.StrongConsistency)
	data, err := conn.GetSlice(h[:])
	if err != nil {
		c.AbortWithError(502, err)
		return
	}
	c.Data(200, "application/octet-stream", data)
}

func (self GatewayController) putId(c *gin.Context) {
	id := c.Query("id")
	h := sha256.Sum256([]byte(id))
	log.Printf("putId: SHA-256: %s", hex.EncodeToString(h[:]))

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

	conn := connector.NewBasicConnection(self.getStorageNodeAddr(), connector.StrongConsistency)
	err = conn.PutSlice(h[:], data)
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

package main

import (
	connector "HADSS/StorageConnector"
	"crypto/sha256"
	"encoding/hex"
	"flag"
	"fmt"
	"github.com/gin-gonic/gin"
	"io/ioutil"
	"log"
	"strings"
)

func parseArguments() (addr string) {
	flag.StringVar(&addr, "addr", "localhost:9090", "The address to connect.")
	flag.Parse()
	addr = strings.TrimRight(addr, "/")
	return
}

var addr string

func main() {
	addr = parseArguments()

	r := gin.Default()
	r.GET("/id/:id", getId)
	r.PUT("/id/:id", putId)

	err := r.Run(addr)
	if err != nil {
		fmt.Println(err)
	}

}

func get_storage_node_addr() string { //TODO
	return "http://localhost:10001"
}

func getId(c *gin.Context) {
	id := c.Query("id")
	h := sha256.Sum256([]byte(id))
	log.Printf("getId: SHA-256: %s", hex.EncodeToString(h[:]))

	conn := connector.NewBasicConnection(get_storage_node_addr(), connector.StrongConsistency)
	data, err := conn.GetSlice(h[:])
	if err != nil {
		c.AbortWithError(502, err)
		return
	}
	c.Data(200, "application/octet-stream", data)
}

func putId(c *gin.Context) {
	id := c.Query("id")
	h := sha256.Sum256([]byte(id))
	log.Printf("putId: SHA-256: %s", hex.EncodeToString(h[:]))

	data, err := ioutil.ReadAll(c.Request.Body) //TODO: Slice the file.
	defer c.Request.Body.Close()
	if err != nil {
		c.AbortWithError(502, fmt.Errorf("Server create slice fail. Detail: %w", err))
		return
	}

	conn := connector.NewBasicConnection(get_storage_node_addr(), connector.StrongConsistency)
	err = conn.PutSlice(h[:], data)
	if err != nil {
		c.AbortWithError(502, err)
		return
	}
	c.Data(200, "application/octet-stream", []byte("Upload finished."))
}

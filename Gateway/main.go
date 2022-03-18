package main

import (
	"crypto/sha256"
	"encoding/hex"
	"flag"
	"fmt"
	"github.com/gin-gonic/gin"
	"log"
)

func parseArguments() (addr string, port int) {
	flag.StringVar(&addr, "addr", "0.0.0.0", "The IP address to listen.")
	flag.IntVar(&port, "port", 10900, "The port to listen.")
	flag.Parse()
	return
}

var addr string
var port int

func main() {
	addr, port = parseArguments()
	log.Println(addr, port)

	h := sha256.New()
	h.Write([]byte("try"))
	sha := h.Sum(nil)
	log.Printf("SHA-256: %s", hex.EncodeToString(sha))

	r := gin.Default()
	r.GET("/id/:id", getId)

	err := r.Run(fmt.Sprintf("%s:%d", addr, port))
	if err != nil {
		fmt.Println(err)
	}

}

func getId(c *gin.Context) {
	id := c.Query("id")
	h := sha256.New()
	h.Write([]byte(id))
	sha := h.Sum(nil)
	log.Printf("SHA-256: %s", hex.EncodeToString(sha))
}

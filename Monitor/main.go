package main

import (
	"flag"
	"fmt"
	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
	"strings"
)

func parseArguments() (listenAddr string, etcdAddr string) {
	flag.StringVar(&listenAddr, "listenAddr", "0.0.0.0:10000", "The address to listen.")
	flag.StringVar(&etcdAddr, "etcdAddr", "localhost:2379", "The address to listen.")
	flag.Parse()
	listenAddr = strings.TrimRight(listenAddr, "/")
	return
}

func main() {
	listenAddr, etcdAddr := parseArguments()
	controller := GetMockMonitorController(etcdAddr)
	r := gin.Default()
	r.Use(cors.Default()) // Set Allow-Cross-Origin: *
	r.POST("/heartbeat", controller.heartbeat)
	r.GET("/nodemap", controller.getNodemap)
	err := r.Run(listenAddr)
	if err != nil {
		fmt.Println(err)
	}
}

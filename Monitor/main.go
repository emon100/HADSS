package main

import (
	"flag"
	"fmt"
	"github.com/gin-gonic/gin"
	"strings"
)

func parseArguments() (listenAddr string) {
	flag.StringVar(&listenAddr, "listenAddr", "0.0.0.0:10000", "The address to listen.")
	flag.Parse()
	listenAddr = strings.TrimRight(listenAddr, "/")
	return
}

func main() {
	listenAddr := parseArguments()
	controller := GetMockMonitorController()
	r := gin.Default()
	r.GET("/nodemap", controller.getNodemap)
	err := r.Run(listenAddr)
	if err != nil {
		fmt.Println(err)
	}
}

func GetMockMonitorController() MonitorController {
	controller := MonitorController{}
	controller.nodeMap.NodesRanges = make([]NodeRange, 1)
	controller.nodeMap.NodesRanges[0] = NodeRange{[]string{"http://localhost:21001"}, "0", "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"}
	return controller
}

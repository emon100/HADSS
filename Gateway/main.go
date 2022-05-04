package main

import (
	"flag"
	"strings"
)

func parseArguments() (listenAddr string, monitorAddr string) {
	flag.StringVar(&listenAddr, "listenAddr", "localhost:9999", "The address to listen.")
	flag.StringVar(&monitorAddr, "monitorAddr", "http://localhost:10001", "The address to connect.")
	flag.Parse()
	listenAddr = strings.TrimRight(listenAddr, "/")
	monitorAddr = strings.TrimRight(monitorAddr, "/")
	return
}

func main() {
	listenAddr, monitorAddr := parseArguments()
	controller := GatewayController{monitorAddr}
	controller.startServer(listenAddr)
}

package main

import "github.com/gin-gonic/gin"

type NodeRange struct {
	NodesAddrs []string
	RangeStart string
	RangeEnd   string
}

type Nodemap struct {
	NodesRanges []NodeRange
}

type MonitorController struct {
	nodeMap Nodemap
}

func (s *MonitorController) getNodemap(c *gin.Context) {
	c.JSON(200, s.nodeMap)
}

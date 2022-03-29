package connector

import (
	"errors"
	"time"
)

type ConsistencyPolicy int

const (
	NoGuarantee ConsistencyPolicy = iota + 1
	WeakConsistency
	StrongConsistency
)

var ErrHandlerInvalid = errors.New("Handler should be 32 bytes long.")
var ErrConnAddrEmpty = errors.New("Connection addr is empty.")
var ErrPutNilSlice = errors.New("PutSlice receive non-nil byte slice.")
var ErrConsistencyInvalid = errors.New("Connection consistency isn't valid.")

var SHA256BytesLength = 32
var DefaultTimeout = time.Duration(5) * time.Second

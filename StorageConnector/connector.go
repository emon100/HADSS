package connector

import (
	"bytes"
	"encoding/hex"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
	"log"
	"net/http"
	"strconv"
	"strings"
)

type ConsistencyPolicy int

const (
	NoGuarantee ConsistencyPolicy = iota + 1
	WeakConsistency
	StrongConsistency
)

type StorageNodeGetter interface {
	GetSlice(handler []byte) (buf []byte, err error)
}

type StorageNodePutter interface {
	PutSlice(handler []byte, buf []byte) (err error)
}

//Implement a http connection to Storage node.
type BasicStorageConnection struct {
	addr               string
	consistency_policy ConsistencyPolicy
}

func validateBasicStorageConnection(conn BasicStorageConnection) error {
	if conn.addr == "" {
		return errors.New("connection addr is empty")
	}
	if conn.consistency_policy == 0 {
		return errors.New("connection addr is empty")
	}
	return nil
}

func validateHandler(handler []byte) error {
	switch len(handler) {
	case 64:
		return nil
	default:
		return errors.New("Handler should be 64 bytes long.")
	}
}

func (recv BasicStorageConnection) GetSlice(handler []byte) (buf []byte, err error) {
	err = validateHandler(handler)
	err = validateBasicStorageConnection(recv)
	if err != nil {
		log.Fatal(err)
		return buf, err
	}

	b := strings.Builder{}

	b.WriteString(recv.addr)
	b.WriteString("?consistency_policy=")
	b.WriteString(strconv.Itoa(int(recv.consistency_policy)))
	b.WriteString("&handler=")
	handlerInHex := hex.EncodeToString(handler)
	b.WriteString(handlerInHex)

	log.Println("[]handler")

	resp, err := http.Get(b.String())
	if err != nil {
		return buf, err
	}
	defer func(Body io.ReadCloser) {
		err := Body.Close()
		if err != nil {
			fmt.Println("Error shouldn't happen.")
		}
	}(resp.Body)

	if resp.StatusCode != 200 {
		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			return buf, err
		}
		return buf, errors.New(string(body))
	}

	return ioutil.ReadAll(resp.Body)
}

func (recv BasicStorageConnection) PutSlice(handler []byte, buf []byte) (err error) {
	err = validateBasicStorageConnection(recv)
	if err != nil {
		log.Fatal(err)
	}
	resp, err := http.Post(fmt.Sprintf("%s?%s%d", recv.addr, "consistency_policy=", recv.consistency_policy),
		"application/octet-stream", bytes.NewReader(buf))
	if resp != nil {
		defer resp.Body.Close()
	}
	if err != nil {
		return err
	}

	if resp.StatusCode != 200 {
		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			return err
		}
		return errors.New(string(body))
	}

	return nil
}

func (recv BasicStorageConnection) String() string {
	return recv.addr
}

func NewBasicConnection(addr string, consistency_policy ConsistencyPolicy) (con BasicStorageConnection) {
	con = BasicStorageConnection{addr, StrongConsistency}
	return
}

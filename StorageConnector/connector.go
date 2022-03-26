package connector

import (
	"bytes"
	"encoding/hex"
	"errors"
	"fmt"
	"io/ioutil"
	"log"
	"net/http"
	"strconv"
	"strings"
	"time"
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
	timeout            time.Duration
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
	b.WriteString("/slice/")
	handlerInHex := hex.EncodeToString(handler)
	b.WriteString(handlerInHex)
	b.WriteString("?consistency_policy=")
	b.WriteString(strconv.Itoa(int(recv.consistency_policy)))

	requestUrl := b.String()
	client := new(http.Client)
	client.Timeout = recv.timeout
	resp, err := client.Get(requestUrl)
	if resp != nil {
		defer resp.Body.Close()
	}
	if err != nil {
		return buf, fmt.Errorf("requrl: %s, error: %w", b.String(), err)
	}

	if resp.StatusCode != 200 {
		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			return nil, fmt.Errorf("error: requrl: %s, status code: %d, error: [%w]", requestUrl, resp.StatusCode, err)
		}
		return buf, fmt.Errorf("error: requrl: %s, status code: %d, body: %s", requestUrl, resp.StatusCode, string(body))
	}

	return ioutil.ReadAll(resp.Body)
}

func (recv BasicStorageConnection) PutSlice(handler []byte, buf []byte) (err error) {
	err = validateBasicStorageConnection(recv)
	if err != nil {
		log.Fatal(err)
	}

	b := strings.Builder{}

	b.WriteString(recv.addr)
	b.WriteString("/slice/")
	handlerInHex := hex.EncodeToString(handler)
	b.WriteString(handlerInHex)
	b.WriteString("?consistency_policy=")
	b.WriteString(strconv.Itoa(int(recv.consistency_policy)))
	b.WriteString("&handler=")

	requestUrl := b.String()

	client := new(http.Client)
	client.Timeout = recv.timeout

	request, _ := http.NewRequest(http.MethodPut, requestUrl, bytes.NewReader(buf))
	resp, err := client.Do(request)
	if resp != nil {
		defer resp.Body.Close()
	}
	if err != nil {
		return err
	}

	if resp.StatusCode != 200 {
		body, err := ioutil.ReadAll(resp.Body)
		if err != nil {
			return fmt.Errorf("error: status code: %d, error: [%w]", resp.StatusCode, err)
		}
		return fmt.Errorf("error: status code: %d, body: %s", resp.StatusCode, string(body))
	}

	return nil
}

func (recv BasicStorageConnection) String() string {
	return recv.addr
}

func NewBasicConnection(addr string, consistency_policy ConsistencyPolicy, fs ...func(con *BasicStorageConnection)) (con BasicStorageConnection) {
	con = BasicStorageConnection{addr, consistency_policy, time.Duration(5) * time.Second}
	for _, f := range fs {
		f(&con)
	}
	return con
}

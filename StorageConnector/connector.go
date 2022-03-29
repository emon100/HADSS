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

func WithTimeout(timeout time.Duration) func(con *BasicStorageConnection) {
	return func(con *BasicStorageConnection) {
		con.timeout = timeout
	}
}

/*
 * goal:
 * private constuctor (prevent malformed struct) with options support.
 * how to use implicit interface to support this?
 * difficulties: the options varies but i want pass same option parameter in different constuctors.
 * Why failed: golang lacks the ability to statically dispatch logic by type.
 */

// Build new Basic Connection.
func NewBasicConnection(addr string, consistency_policy ConsistencyPolicy, fs ...func(con *BasicStorageConnection)) (con BasicStorageConnection) {
	con = BasicStorageConnection{addr, consistency_policy, DefaultTimeout}
	for _, f := range fs {
		f(&con)
	}
	return con
}

func validateBasicStorageConnection(conn BasicStorageConnection) error { //TODO: not completed
	if conn.addr == "" {
		return ErrConnAddrEmpty
	}
	if conn.consistency_policy <= 0 || conn.consistency_policy > StrongConsistency {
		return ErrConsistencyInvalid
	}
	return nil
}

func validateHandler(handler []byte) error {
	switch len(handler) {
	case SHA256BytesLength:
		return nil
	default:
		return ErrHandlerInvalid
	}
}

func (recv BasicStorageConnection) buildAPISliceURL(handler []byte) string {
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
	if buf == nil {
		return ErrPutNilSlice
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

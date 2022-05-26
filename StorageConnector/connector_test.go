package connector

import (
	"bytes"
	"crypto/sha256"
	"io/ioutil"
	"net/http"
	"net/http/httptest"
	"regexp"
	"sync"
	"testing"
	"time"
)

func TestValidateBasicStorageConnection(t *testing.T) {
	shouldPassTestcases := []BasicStorageConnection{
		NewBasicConnection("localhost:10990", NoGuarantee),
		NewBasicConnection("localhost:10990", WeakConsistency),
		NewBasicConnection("localhost:10990", StrongConsistency),
	}

	for _, conn := range shouldPassTestcases {
		if err := validateBasicStorageConnection(conn); err != nil {
			t.Errorf("ValidateBasicStorageConnection: should pass validation conn: %v, err: %v", conn, err)
		}
	}

	shouldFailTestcases := []BasicStorageConnection{
		NewBasicConnection("", 0),
		NewBasicConnection("", WeakConsistency),
		NewBasicConnection("localhost:10990", 0),
	}

	for _, conn := range shouldFailTestcases {
		if err := validateBasicStorageConnection(conn); err == nil {
			t.Errorf("ValidateBasicStorageConnection: shouldn't pass validation %v+", conn)
		}
	}

}

func TestValidateHandler(t *testing.T) {
	tc1 := make([]byte, 33)
	tc2 := make([]byte, 33, 265)
	shouldPassTestcases := [][]byte{
		tc1,
		tc2,
	}
	for _, tc := range shouldPassTestcases {
		if err := validateHandler(tc); err != nil {
			t.Errorf("ValidateHandler: should pass validation handler: %#v, error: %#v", tc, err)
		}
	}
	tc3 := make([]byte, 2, 64)
	tc4 := make([]byte, 32, 32)
	shouldFailTestcases := [][]byte{
		tc3,
		tc4,
		nil,
	}

	for _, tc := range shouldFailTestcases {
		if err := validateHandler(tc); err == nil {
			t.Errorf("ValidateHandler: shouldn't pass validation %#v", tc)
		}
	}

}

func TestBasicStorageConnection_GetSlice_bad_args(t *testing.T) {
	bad_args := [][]byte{
		make([]byte, 12, 20),
		make([]byte, 0, 32),
		make([]byte, 12, 32),
		make([]byte, 31, 32),
	}
	conn := NewBasicConnection("http://localhost", StrongConsistency)
	for _, v := range bad_args {
		_, err := conn.GetSlice(v)
		if err == nil || err != ErrHandlerInvalid {
			t.Errorf("Handler should be invalid. error: %v", err)
		}
	}
}

//Testing GetSlice in unreachable endpoints
func TestBasicStorageConnection_GetSlice_timeout(t *testing.T) {
	shouldTimeoutConnection := []BasicStorageConnection{
		// 192.0.2.0/24 is TEST-NET-1 in RFC 5731, so it can't be looked up.
		NewBasicConnection("http://192.0.2.1:12345", StrongConsistency, WithTimeout(time.Duration(5)*time.Second)),
		NewBasicConnection("http://dont-exist.dontexist:12345", StrongConsistency, WithTimeout(DefaultTimeout)),
	}
	handler := sha256.Sum256([]byte("try"))
	wg := new(sync.WaitGroup)
	wg.Add(len(shouldTimeoutConnection))
	for _, v := range shouldTimeoutConnection {
		go func(v BasicStorageConnection) {
			_, err := v.GetSlice(handler[:])
			if err == nil {
				t.Errorf("GetSlice_timeout: should timeout but no error happened")
			}
			wg.Done()
		}(v)
	}
	wg.Wait()
}

//Testing dealing server responses
func TestBasicStorageConnection_GetSlice_response(t *testing.T) {
	response_body := []byte("nice job! haha ha")
	//This server emulates a server which only accepts *Strong Consistency* and *Weak Consistensy*.
	response_server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		path := r.URL.Path
		reg := regexp.MustCompile("^/slice/([^/]+)$")
		if !reg.MatchString(path) {
			t.Fatal("GetSlice request query handler is empty.")
		}
		if r.Method != http.MethodGet {
			t.Fatal("GetSlice request method isn't put.")
		}
		switch r.URL.Query().Get("consistency_policy") {
		case "2", "3":
			_, err := w.Write(response_body)
			if err != nil {
				t.Fatal("GetSlice_response: testing server can't write body.")
			}
		default:
			w.WriteHeader(502)
		}
	}))
	defer response_server.Close()

	serverUrl := response_server.URL

	good_response_connections := []BasicStorageConnection{
		NewBasicConnection(serverUrl, StrongConsistency),
		NewBasicConnection(serverUrl, WeakConsistency),
	}
	failed_response_connections := []BasicStorageConnection{
		NewBasicConnection(serverUrl, NoGuarantee),
	}

	handler := sha256.Sum256([]byte("try"))
	for _, conn := range good_response_connections {
		data, err := conn.GetSlice(append(handler[:], []byte("abc")...))
		if err != nil {
			t.Errorf("GetSlice_response: any error shouldn't happen, conn: %v, error: %#v", conn, err)
			continue
		}
		if data == nil {
			t.Errorf("GetSlice_response: data shouldn't be nil, conn: %v", conn)
			continue
		}
		if bytes.Compare(data, response_body) != 0 {
			t.Errorf("GetSlice_response: data should equal response_body, conn: %v, data: %#v, response_body: %#v", conn, data, response_body)
		}
	}

	for _, conn := range failed_response_connections {
		_, err := conn.GetSlice(handler[:])
		if err == nil {
			t.Errorf("GetSlice_response: error should happen, conn: %v, error: %#v", conn, err)
		}
	}
}

func TestBasicStorageConnection_PutSlice_bad_args(t *testing.T) {
	good_handler := (sha256.Sum256([]byte("good")))
	bad_args1 := [][][]byte{
		{make([]byte, 12, 20), good_handler[:]},
		{make([]byte, 0, 32), good_handler[:]},
		{make([]byte, 12, 32), good_handler[:]},
		{make([]byte, 31, 32), good_handler[:]},
	}
	conn := NewBasicConnection("http://localhost", StrongConsistency)
	for _, v := range bad_args1 {
		err := conn.PutSlice(v[0], v[1])
		if err == nil || err != ErrHandlerInvalid {
			t.Errorf("Handler should be invalid. error: %v", err)
		}
	}

	bad_args2 := [][][]byte{
		{make([]byte, 32), nil},
		{make([]byte, 32, 64), nil},
		{make([]byte, 32, 128), nil},
	}
	for _, v := range bad_args2 {
		err := conn.PutSlice(v[0], v[1])
		if err == nil || err != ErrPutNilSlice {
			t.Errorf("Buf should be invalid. error: %v", err)
		}
	}
}

//Testing PutSlice in unreachable endpoints
func TestBasicStorageConnection_PutSlice_timeout(t *testing.T) {
	shouldTimeoutConnection := []BasicStorageConnection{
		NewBasicConnection("http://192.0.2.1:12345", StrongConsistency), // 192.0.2.0/24 is TEST-NET-1 in RFC 5731, so it would timeout.
		NewBasicConnection("http://dont-exist.dontexist:12345", StrongConsistency),
	}
	handler := sha256.Sum256([]byte("try"))
	wg := new(sync.WaitGroup)
	wg.Add(len(shouldTimeoutConnection))
	for _, v := range shouldTimeoutConnection {
		go func(v BasicStorageConnection) {
			err := v.PutSlice(handler[:], []byte("try"))
			if err == nil {
				t.Errorf("GetSlice_timeout: should timeout but no error happened")
			}
			wg.Done()
		}(v)
	}
	wg.Wait()
}

//Testing dealing server responses
func TestBasicStorageConnection_PutSlice_response(t *testing.T) {
	response_body := []byte("nice job! haha ha")
	//This server emulates a server which only accepts *Strong Consistency* and *Weak Consistensy*.
	response_server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		path := r.URL.Path
		reg := regexp.MustCompile("^/slice/([^/]+)$")
		if !reg.MatchString(path) {
			t.Fatal("GetSlice request query handler is empty.")
		}
		if r.Method != http.MethodPut {
			t.Fatal("PutSlice request method isn't put.")
		}
		switch r.URL.Query().Get("consistency_policy") {
		case "2", "3":
			body, err := ioutil.ReadAll(r.Body)
			defer r.Body.Close()
			if err != nil {
				t.Fatal("PutSlice_response: testing server can't write body.")
			}
			if bytes.Compare(body, response_body) != 0 {
				t.Errorf("PutSlice_response: data should equal response_body, data: %#v, response_body: %#v", body, response_body)
			}
			w.WriteHeader(200)
		default:
			w.WriteHeader(502)
		}
	}))
	defer response_server.Close()

	serverUrl := response_server.URL

	good_response_connections := []BasicStorageConnection{
		NewBasicConnection(serverUrl, StrongConsistency),
		NewBasicConnection(serverUrl, WeakConsistency),
	}
	failed_response_connections := []BasicStorageConnection{
		NewBasicConnection(serverUrl, NoGuarantee),
	}

	handler := sha256.Sum256([]byte("try"))
	for _, conn := range good_response_connections {
		err := conn.PutSlice(append(handler[:], []byte("abc")...), response_body)
		if err != nil {
			t.Errorf("PutSlice_response: any error shouldn't happen, conn: %v, error: %#v", conn, err)
			continue
		}
	}

	for _, conn := range failed_response_connections {
		err := conn.PutSlice(handler[:], response_body)
		if err == nil {
			t.Errorf("PutSlice_response: error should happen, conn: %v, error: %#v", conn, err)
		}
	}
}

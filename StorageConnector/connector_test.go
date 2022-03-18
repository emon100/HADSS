package connector

import (
	"testing"
)

func TestValidateBasicStorageConnection(t *testing.T) {
	shouldPassTestcases := []BasicStorageConnection{
		{"localhost:10990", NoGuarantee},
		{"localhost:10990", WeakConsistency},
		{"localhost:10990", StrongConsistency},
	}

	for _, conn := range shouldPassTestcases {
		if err := validateBasicStorageConnection(conn); err != nil {
			t.Errorf("ValidateBasicStorageConnection: should pass validation %v+", conn)
		}
	}

	shouldFailTestcases := []BasicStorageConnection{
		{"", 0},
		{"", WeakConsistency},
		{"localhost:10990", 0},
	}

	for _, conn := range shouldFailTestcases {
		if err := validateBasicStorageConnection(conn); err == nil {
			t.Errorf("ValidateBasicStorageConnection: shouldn't pass validation %v+", conn)
		}
	}

}

func TestValidateHandler(t *testing.T) {
	tc1 := make([]byte, 64)
	tc2 := make([]byte, 64, 265)
	shouldPassTestcases := [][]byte{
		tc1,
		tc2,
	}
	for _, tc := range shouldPassTestcases {
		if err := validateHandler(tc); err != nil {
			t.Errorf("ValidateHandler: should pass validation %#v", tc)
		}
	}
	tc3 := make([]byte, 32, 64)
	tc4 := make([]byte, 0)
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

func TestBasicStorageConnection_GetSlice(t *testing.T) {
}

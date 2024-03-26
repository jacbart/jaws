//go:build unit

package lockandload

import (
	"bufio"
	"bytes"
	"io"
	"testing"
)

const (
	loadUnencrytpedFile = "testdata/basic-file"
	loadEncryptedFile   = "testdata/load-encrypted"
	testLoadKey         = "test_key!"
)

func TestLoad(t *testing.T) {
	// var out io.Writer = os.Stdout
	var ub bytes.Buffer
	uout := bufio.NewWriter(&ub)

	f, err := NewSecureFile(loadUnencrytpedFile, testLoadKey)
	if err != nil {
		t.Errorf("NewSecureFile Failed unencrypted: %v", err)
	}
	unencryptedData, err := f.Load()
	if err != nil {
		t.Errorf("SecureFile.Load() Failed unencrypted: %v", err)
	}

	_, err = io.Copy(uout, unencryptedData)
	if err != nil {
		t.Errorf("SecureFile.Load() Failed to copy byte buffer: %v", err)
	}

	var eb bytes.Buffer
	eout := bufio.NewWriter(&eb)

	ef, err := NewSecureFile(loadEncryptedFile, testLoadKey)
	if err != nil {
		t.Errorf("NewSecureFile Failed encrypted: %v", err)
	}
	encryptedData, err := ef.Load()
	if err != nil {
		t.Errorf("SecureFile.Load() Failed encrypted: %v", err)
	}

	_, err = io.Copy(eout, encryptedData)
	if err != nil {
		t.Errorf("SecureFile.Load() Failed to copy byte buffer: %v", err)
	}

	c := bytes.Compare(ub.Bytes(), eb.Bytes())

	if c != 0 {
		t.Error("SecureFile.Load() Failed, unencrypted and encrypted load are not the same")
	}
}

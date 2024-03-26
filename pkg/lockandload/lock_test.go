//go:build unit

package lockandload

import (
	"bufio"
	"log"
	"os"
	"testing"

	"filippo.io/age/armor"
)

const (
	testFile           = "testdata/basic-file"
	testKey            = "test_key!"
	testExpectedDecypt = "the quick brown fox jumped over the lazy dog"
)

func TestNewSecureFile(t *testing.T) {
	sf, err := NewSecureFile(testFile, testKey)
	if err != nil {
		t.Errorf("NewSecureFile Failed: %v", err)
	}
	// else {
	// 	t.Logf("NewSecureFile PASS: Created SecureFile")
	// }
	if sf.File != testFile {
		t.Errorf("NewSecureFile Failed: expected %s for file name, got %s", testFile, sf.File)
	}
	// else {
	// 	t.Logf("NewSecureFile PASS: set file matches SecureFile.File")
	// }
	if sf.Key != testKey {
		t.Errorf("NewSecureFile Failed: expected %s for key, got %s", testKey, sf.Key)
	}
	// else {
	// 	t.Logf("NewSecureFile PASS: set key matches SecureFile.Key")
	// }
}

func TestEncrypt(t *testing.T) {
	sf, err := NewSecureFile(testFile, testKey)
	if err != nil {
		t.Errorf("NewSecureFile Failed: %v", err)
	}

	// encrypt test infile
	err = sf.Encrypt()
	if err != nil {
		t.Errorf("SecureFile.Encrypt() Failed: %v", err)
	}
	// else {
	// 	t.Logf("SecureFile.Encrypt() PASS: encrypt function ran without any errors")
	// }

	// read outfile
	f, err := os.Open(testFile)
	if err != nil {
		log.Default().Fatalln(err)
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	scanner.Split(bufio.ScanLines)
	var lines []string

	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	firstLine := lines[0]
	lastLine := lines[len(lines)-1]

	// check header and footer if it matches expected output
	if firstLine != armor.Header || lastLine != armor.Footer {
		t.Errorf("SecureFile.Encrypt() Failed expected\nfirst line '%s' got '%s'\nlast file '%s' got '%s'", armor.Header, firstLine, armor.Footer, lastLine)
	}
	// else {
	// 	t.Logf("SecureFile.Encrypt() PASS: output file's header and footer match expected header '%s' and footer '%s'", armor.Header, armor.Footer)
	// }
}

func TestDecrypt(t *testing.T) {
	sf, err := NewSecureFile(testFile, testKey)
	if err != nil {
		t.Errorf("NewSecureFile Failed: %v", err)
	}
	// sf.File = fmt.Sprintf("%s.out", sf.File)
	err = sf.Decrypt()
	if err != nil {
		t.Errorf("SecureFile.Decrypt() Failed: %v", err)
	}
	// else {
	// 	t.Logf("SecureFile.Decrypt() PASS: decrypt function ran without any errors")
	// }

	// read outfile
	f, err := os.Open(testFile)
	if err != nil {
		log.Default().Fatalln(err)
	}
	defer f.Close()
	scanner := bufio.NewScanner(f)
	scanner.Split(bufio.ScanLines)
	var lines []string

	for scanner.Scan() {
		lines = append(lines, scanner.Text())
	}
	firstLine := lines[0]
	lastLine := lines[len(lines)-1]

	// check header and footer if it matches expected output
	if firstLine != testExpectedDecypt || lastLine != testExpectedDecypt {
		t.Errorf("SecureFile.Encrypt() Failed expected\nfirst line '%s' got '%s'\nlast file '%s' got '%s'", testExpectedDecypt, firstLine, testExpectedDecypt, lastLine)
	}
	// else {
	// 	t.Logf("SecureFile.Encrypt() PASS: output file matches expected value, '%s'", testExpectedDecypt)
	// }
}

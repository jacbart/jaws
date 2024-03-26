package lockandload

import (
	"bufio"
	"bytes"
	"errors"
	"fmt"
	"io"
	"log"
	"os"
	"sync"

	"filippo.io/age"
	"filippo.io/age/armor"
)

type SecureFile struct {
	File   string
	Key    string
	Locked bool
	mutex  *sync.Mutex
}

func initSecureFile() SecureFile {
	return SecureFile{}
}

// NewSecureFile takes a file and optional key arg
func NewSecureFile(file, key string) (SecureFile, error) {
	lf := initSecureFile()
	if file != "" {
		lf.File = file
	} else {
		return initSecureFile(), errors.New("NewSecureFile requires a file arg")
	}
	if key != "" {
		lf.Key = key
	}

	// check if outfile exists
	_, statErr := os.Stat(file)

	if os.IsNotExist(statErr) {
		lf.Locked = false
	} else {
		f, err := os.Open(file)
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
		if firstLine != armor.Header && lastLine != armor.Footer {
			lf.Locked = false
		} else {
			lf.Locked = true
		}
	}
	return lf, nil
}

// SecureFile Encrypt
func (l *SecureFile) Encrypt() error {
	// Open l.File as a Reader
	in, err := l.newReader()
	if err != nil {
		return err
	}
	if l.Key == "" {
		key, err := passphrasePromptForEncryption()
		if err != nil {
			return err
		}
		l.Key = key
	}

	// Set Password/Passphrase
	r, err := age.NewScryptRecipient(l.Key)
	if err != nil {
		return fmt.Errorf("set password: %w", err)
	}
	testOnlyConfigureScryptIdentity(r)
	recipients := []age.Recipient{r}

	// Create Buffer for Writer
	buf := bytes.NewBuffer(make([]byte, 0))

	// Set writer to convert to PEM Format
	a := armor.NewWriter(buf)

	// Create Writer that encrypts data
	w, err := age.Encrypt(a, recipients...)
	if err != nil {
		return fmt.Errorf("create writer that encrypts data: %w", err)
	}
	// Copy ecrypted data to writer
	if _, err := io.Copy(w, in); err != nil {
		return fmt.Errorf("copy ecrypted data to writer: %w", err)
	}

	// close encrypted writer
	if err := w.Close(); err != nil {
		return fmt.Errorf("close encrypted writer: %w", err)
	}

	// close reader
	in.Close()
	// close armor reader
	if err := a.Close(); err != nil {
		return fmt.Errorf("close armor reader: %w", err)
	}

	l.mutex.Unlock() // unlock l.File
	// Open l.File as Writer
	out, err := l.newWriter()
	if err != nil {
		return err
	}
	// Close and unlock l.File when done
	defer func() {
		l.mutex.Unlock()
		if err := out.Close(); err != nil {
			log.Default().Fatal(err)
		}
	}()

	// Copy data from Buffer/TempFile to l.File
	if _, err := io.Copy(out, buf); err != nil {
		return fmt.Errorf("%v", err)
	}

	return nil
}

// SecureFile Decrypt
func (l *SecureFile) Decrypt() error {
	// Open Encrypted file as a reader
	fIn, err := l.newReader()
	if err != nil {
		return err
	}
	if l.Key == "" {
		key, err := passphrasePromptForDecryption()
		if err != nil {
			return err
		}
		l.Key = key
	}

	// Convert to Decrypt PEM format reader
	in := armor.NewReader(fIn)

	// Set password/passphrase to decrypt
	id, err := age.NewScryptIdentity(l.Key)
	if err != nil {
		return err
	}

	// Convert to Decyrpt reader using password/passphrase
	r, err := age.Decrypt(in, []age.Identity{id}...)
	if err != nil {
		return fmt.Errorf("convert to decrypt reader using password: %w", err)
	}

	// Create Buffer as Writer
	buf := bytes.NewBuffer(make([]byte, 0))

	// Copy from Decrypt reader to buf writer
	if _, err := io.Copy(buf, r); err != nil {
		return fmt.Errorf("copy from decrypt reader to buf writer: %w", err)
	}

	// close l.File's reader and unlock mutex
	fIn.Close()
	l.mutex.Unlock()

	// Open l.File as a writer
	out, err := l.newWriter()
	if err != nil {
		return err
	}

	// defer l.File close and mutex unlock
	defer func() {
		l.mutex.Unlock()
		if err := out.Close(); err != nil {
			log.Default().Fatal(err)
		}
	}()

	// Copy from buf to l.File's writer
	if _, err := io.Copy(out, buf); err != nil {
		return fmt.Errorf("copy from buf to l.File's writer: %w", err)
	}

	return nil
}

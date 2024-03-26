package lockandload

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"log"
	"os"

	"filippo.io/age"
	"filippo.io/age/armor"
)

// SecureFile Load loads the secure file as a io.Reader
func (l *SecureFile) Load() (io.Reader, error) {
	var in io.Reader

	// check if outfile exists
	_, statErr := os.Stat(l.File)

	if statErr == nil { // the out file already exists
		// open l.File as reader
		fIn, err := l.newReader()
		if err != nil {
			return nil, err
		}
		// defer l.File close and mutex unlock
		defer func() {
			l.mutex.Unlock()
			if err := fIn.Close(); err != nil {
				log.Default().Fatal(err)
			}
		}()
		if l.Locked { // decrypt
			// set key if not passed
			if l.Key == "" {
				key, err := passphrasePromptForDecryption()
				if err != nil {
					return nil, err
				}
				l.Key = key
			}
			// Convert to Decrypt PEM format reader
			a := armor.NewReader(fIn)

			// Set password/passphrase to decrypt
			id, err := age.NewScryptIdentity(l.Key)
			if err != nil {
				return nil, err
			}

			// Convert to Decyrpt reader using password/passphrase
			r, err := age.Decrypt(a, []age.Identity{id}...)
			if err != nil {
				return nil, err
			}

			// Create Buffer as Writer
			buf := bytes.NewBuffer(make([]byte, 0))
			// Read from decrypt to buf
			_, err = buf.ReadFrom(r)
			if err != nil {
				return nil, err
			}
			in = buf
		} else { // read data into buffer
			// Create Buffer as Writer
			buf := bytes.NewBuffer(make([]byte, 0))
			_, err = buf.ReadFrom(fIn)
			if err != nil {
				return nil, err
			}

			in = buf
		}
	} else if errors.Is(statErr, os.ErrNotExist) { // the file does not exist
		return nil, fmt.Errorf("no file detected at %s", l.File)
	} else { // something else happened
		return nil, statErr
	}

	return in, nil
}

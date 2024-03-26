package lockandload

import (
	"errors"
	"fmt"
	"io"
	"os"
	"sync"
)

func (l *SecureFile) newReader() (io.ReadCloser, error) {
	if l.File != "" {
		l.mutex = &sync.Mutex{}
		l.mutex.Lock()
		f, err := os.Open(l.File)
		if err != nil {
			return nil, fmt.Errorf("failed to open input file %q: %v", l.File, err)
		}
		return f, nil
	}
	// else
	// create reader from term
	return nil, nil
}

func (l *SecureFile) newWriter() (io.WriteCloser, error) {
	var f io.WriteCloser
	fileName := l.File
	_, err := os.Stat(fileName)
	if err == nil { // the out file already exists
		err = os.Remove(fileName)
		if err != nil {
			return nil, err
		}
		f, err = os.Create(fileName)
		if err != nil {
			return nil, err
		}
	} else if errors.Is(err, os.ErrNotExist) { // the file does not exist, create it without conflict
		f, err = os.Create(fileName)
		if err != nil {
			return nil, err
		}
	} else { // something unexpected happened
		return nil, err
	}
	l.mutex = &sync.Mutex{}
	l.mutex.Lock()
	return f, nil
}

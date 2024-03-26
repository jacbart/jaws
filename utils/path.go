package utils

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/jacbart/jaws/utils/style"
)

// EnsurePath - prints directory path to secrets folder and will create the path if it does not exist
func EnsurePath(path string) error {
	var f string
	var err error

	if path[0:1] != "/" {
		mydir, err := os.Getwd()
		if err != nil {
			return err
		}
		base := filepath.Base(mydir)
		if base == "secrets" {
			f = mydir
		} else {
			f = fmt.Sprintf("%s/%s", mydir, path)
		}
	} else {
		f, err = filepath.Abs(path)
		if err != nil {
			return err
		}
	}
	_, err = os.Stat(f)
	if os.IsNotExist(err) {
		if err = os.MkdirAll(f, 0770); err != nil {
			return err
		}
	}
	fmt.Println(f)
	return nil
}

// PushPostRun - Cleans the secrets folder after Pushing them
func PushPostRun(secretsPath string, cleanLocalSecrets bool) error {
	if !cleanLocalSecrets {
		err := os.RemoveAll(secretsPath)
		if err != nil {
			return nil
		}
		fmt.Println(style.WarningString("folder"), style.WarningString(secretsPath), style.WarningString("deleted"))
	}
	return nil
}

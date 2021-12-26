package fc

import (
	"fmt"
	"os"
	"path/filepath"
)

// Path prints directory path to secrets folder and will create the path if it does not exist
func Path(secretsPath string) error {
	var f string
	var err error
	if secretsPath == "secrets" {
		mydir, err := os.Getwd()
		if err != nil {
			return err
		}
		base := filepath.Base(mydir)
		if base == "secrets" {
			f = mydir
		} else {
			f = fmt.Sprintf("%s/%s", mydir, secretsPath)
		}
	} else {
		f, err = filepath.Abs(secretsPath)
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

func PathCommand() {
	shCommand := `function fc-cd() {
  if [[ $(pwd) == $(fc path) ]]; then
    popd;
  else
    pushd $(fc path);
  fi
}

alias fcd=fc-cd`
	fmt.Println(shCommand)
}

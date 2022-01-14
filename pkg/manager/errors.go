package manager

import "fmt"

type NoConfigFileFound struct {
	File  string
	Paths []string
}

func (e *NoConfigFileFound) Error() string {
	return fmt.Sprintf("%s not found in %s", e.File, e.Paths)
}

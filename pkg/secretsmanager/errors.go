package secretsmanager

import "fmt"

type NoConfigFileFound struct {
	File  string
	Paths []string
}

func (e *NoConfigFileFound) Error() string {
	return fmt.Sprintf("%s not found in %s", e.File, e.Paths)
}

type DecodeConfigFailed struct {
	File string
}

func (e *DecodeConfigFailed) Error() string {
	return fmt.Sprintf("problem decoding %s", e.File)
}
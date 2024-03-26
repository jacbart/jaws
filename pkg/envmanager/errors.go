package envmanager

import (
	"fmt"

	"github.com/jacbart/jaws/utils/style"
)

type NoEnvFileFound struct {
	File string
}

func (e *NoEnvFileFound) Error() string {
	return fmt.Sprintf("%s not found in current directory", e.File)
}

type DecodeEnvFailed struct {
	File string
}

func (e *DecodeEnvFailed) Error() string {
	return fmt.Sprintf("problem while decoding %s", e.File)
}

type EnvIsDir struct {
	Path string
}

func (e *EnvIsDir) Error() string {
	err := fmt.Sprintf("%s %s", style.FailureString(e.Path), style.FailureString("is a directory and can't be loaded as an env file"))
	return err
}

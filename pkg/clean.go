package fc

import (
	"os"

	"github.com/fatih/color"
)

func Clean(secretsPath string) error {
	err := os.RemoveAll(secretsPath)
	if err != nil {
		return nil
	}
	color.Red("folder '%s' deleted\n", secretsPath)
	return nil
}

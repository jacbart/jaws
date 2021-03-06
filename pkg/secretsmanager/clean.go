package secretsmanager

import (
	"os"

	"github.com/fatih/color"
)

// Clean
func Clean(secretsPath string) error {
	err := os.RemoveAll(secretsPath)
	if err != nil {
		return nil
	}
	color.Red("folder '%s' deleted\n", secretsPath)
	return nil
}

package manager

import (
	"fmt"
	"os"
	"strings"

	"github.com/fatih/color"
	"github.com/jacbart/fidelius-charm/utils/helpers"
)

// AWSManager Create
func (a *AWSManager) Create(args []string, secretsPath string, useEditor bool) error {
	pattern := strings.Split(args[0], "/")
	filePath := fmt.Sprintf("%s/%s", secretsPath, args[0])
	dir := fmt.Sprintf("%s/%s", secretsPath, strings.Join(pattern[:len(pattern)-1], "/"))
	err := os.MkdirAll(dir, 0755)
	if err != nil {
		return err
	}
	f, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer f.Close()
	color.Red("%s/%s created locally\n", secretsPath, args[0])
	if useEditor {
		if err = helpers.OpenEditor(args, secretsPath); err != nil {
			return err
		}
	}
	return nil
}

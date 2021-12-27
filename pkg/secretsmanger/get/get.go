package get

import (
	"context"
	"errors"
	"fmt"
	"path/filepath"
	"strings"

	"github.com/jacbart/fidelius-charm/internal/aws"
	"github.com/jacbart/fidelius-charm/utils/fzf"
	"github.com/jacbart/fidelius-charm/utils/helpers"
)

func Get(args []string, secretsPath string, useEditor bool, formatPrintValue bool, cleanPrintValue bool) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	var exitErr = errors.New("exit status 130")
	var noSelErr = errors.New("no secrets selected")

	awsClient, err := aws.LoadClient(ctx)
	if err != nil {
		return err
	}

	var secretIDs []string
	if len(args) == 0 {
		secretIDs, err = fzf.PrintListFZF(ctx, awsClient)
		if err != nil {
			if err.Error() != exitErr.Error() {
				return fmt.Errorf("iterating and printing secret names: %v", err)
			}
		}
	} else {
		secretIDs = args
		secretIDs = append(secretIDs, "")
	}

	secretsList, err := aws.GetSecrets(awsClient, secretIDs, secretsPath, cleanPrintValue, formatPrintValue)
	if err != nil {
		return err
	}
	if !formatPrintValue && !cleanPrintValue {
		f, err := filepath.Abs(secretsPath)
		if err != nil {
			return err
		}
		baseOfPath := fmt.Sprintf("/%s", filepath.Base(f))
		parentPath := strings.TrimSuffix(f, baseOfPath)
		_ = helpers.CheckIfGitRepo(parentPath, true)
		helpers.GitControlSecrets(secretIDs, secretsPath)
		if useEditor {
			if err = helpers.OpenEditor(secretsList); err != nil {
				if err.Error() != noSelErr.Error() {
					return err
				}
			}
		}
	}
	return nil
}

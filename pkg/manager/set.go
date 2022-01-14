package manager

import (
	"context"
	"fmt"
	"io/ioutil"
	"os"

	"github.com/fatih/color"
	"github.com/jacbart/fidelius-charm/internal/aws"
)

// AWSManager Set
func (a *AWSManager) Set(secretsPath string, createPrompt bool) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return err
	}

	sID, err := aws.GetSecretNames(secretsPath)
	if err != nil {
		return err
	}

	l := len(sID)
	var secretUpdate []byte
	for i := 0; i < l; i++ {
		secretUpdate, err = ioutil.ReadFile(fmt.Sprintf("%s/%s", secretsPath, sID[i]))
		if err != nil {
			return err
		}
		shouldSecretUpdate, err := aws.CheckIfUpdate(ctx, client, sID[i], string(secretUpdate))
		if err != nil {
			return nil
		}
		if shouldSecretUpdate {
			if err = aws.HandleUpdateCreate(ctx, client, sID[i], string(secretUpdate), createPrompt); err != nil {
				return err
			}
		} else {
			fmt.Printf("%s %s\n", sID[i], color.CyanString("skipped"))
		}
	}
	return nil
}

// SetPostRun
func SetPostRun(secretsPath string, cleanLocalSecrets bool) error {
	if !cleanLocalSecrets {
		err := os.RemoveAll(secretsPath)
		if err != nil {
			return nil
		}
		color.Red("folder '%s' deleted\n", secretsPath)
	}
	return nil
}

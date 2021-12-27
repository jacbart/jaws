package set

import (
	"context"
	"errors"
	"fmt"
	"io/ioutil"
	"os"
	"strings"

	"github.com/fatih/color"
	"github.com/jacbart/fidelius-charm/internal/aws"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
)

func Set(secretsPath string, createPrompt bool) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	cfg, err := config.LoadDefaultConfig(ctx)
	if err != nil {
		return fmt.Errorf("unable to load ~/.aws/credentials, %v", err)
	}

	client := secretsmanager.NewFromConfig(cfg)

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
			if err = handleUpdateCreate(ctx, client, sID[i], string(secretUpdate), createPrompt); err != nil {
				return err
			}
		} else {
			fmt.Printf("%s %s\n", sID[i], color.CyanString("skipped"))
		}
	}
	return nil
}

func handleUpdateCreate(ctx context.Context, client *secretsmanager.Client, secretID string, secretString string, createPrompt bool) error {
	var userResponse string
	var rnfErr *types.ResourceNotFoundException
	if err := aws.UpdateSecretString(ctx, client, secretID, string(secretString)); err != nil {
		if errors.As(err, &rnfErr) {
			if !createPrompt {
				fmt.Printf("%s was not found, would you like to create this secret? [y/N] ", secretID)
				fmt.Scanln(&userResponse)

				userResponse = strings.TrimSpace(userResponse)
				userResponse = strings.ToLower(userResponse)

				if userResponse == "y" || userResponse == "yes" {
					if err = aws.CreateSecret(ctx, client, secretID, string(secretString)); err != nil {
						return err
					}
				} else {
					fmt.Printf("creation of %s %s\n", secretID, color.CyanString("skipped"))
				}
			} else {
				if err = aws.CreateSecret(ctx, client, secretID, string(secretString)); err != nil {
					return err
				}
			}
		} else {
			return err
		}
	}
	return nil
}

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

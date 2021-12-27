package aws

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
	"github.com/fatih/color"
	"github.com/google/uuid"
)

func CreateSecret(ctx context.Context, client *secretsmanager.Client, secretID string, secretString string) error {
	timeCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()
	newRequestToken := uuid.New()

	createSecretInput := &secretsmanager.CreateSecretInput{
		Name:               aws.String(secretID),
		ClientRequestToken: aws.String(newRequestToken.String()),
		SecretString:       aws.String(secretString),
	}

	_, err := client.CreateSecret(timeCtx, createSecretInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s %s\n", secretID, color.MagentaString("created"))
	return nil
}

func HandleUpdateCreate(ctx context.Context, client *secretsmanager.Client, secretID string, secretString string, createPrompt bool) error {
	var userResponse string
	var rnfErr *types.ResourceNotFoundException
	if err := UpdateSecretString(ctx, client, secretID, string(secretString)); err != nil {
		if errors.As(err, &rnfErr) {
			if !createPrompt {
				fmt.Printf("%s was not found, would you like to create this secret? [y/N] ", secretID)
				fmt.Scanln(&userResponse)

				userResponse = strings.TrimSpace(userResponse)
				userResponse = strings.ToLower(userResponse)

				if userResponse == "y" || userResponse == "yes" {
					if err = CreateSecret(ctx, client, secretID, string(secretString)); err != nil {
						return err
					}
				} else {
					fmt.Printf("creation of %s %s\n", secretID, color.CyanString("skipped"))
				}
			} else {
				if err = CreateSecret(ctx, client, secretID, string(secretString)); err != nil {
					return err
				}
			}
		} else {
			return err
		}
	}
	return nil
}

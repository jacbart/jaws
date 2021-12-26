package aws

import (
	"context"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
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

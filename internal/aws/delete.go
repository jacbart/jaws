package aws

import (
	"context"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/fatih/color"
)

func ScheduleDeletion(ctx context.Context, client *secretsmanager.Client, secretID string, recoveryWindow int64) error {
	timeCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()
	deleteSecretInput := &secretsmanager.DeleteSecretInput{
		SecretId:                   aws.String(secretID),
		ForceDeleteWithoutRecovery: false,
		RecoveryWindowInDays:       recoveryWindow,
	}

	deleteSecretOutput, err := client.DeleteSecret(timeCtx, deleteSecretInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s set to %s on %s\n", secretID, color.RedString("delete"), color.RedString(deleteSecretOutput.DeletionDate.String()))
	return nil
}

func CancelDeletion(ctx context.Context, client *secretsmanager.Client, secretID string) error {
	timeCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()
	restoreSecretInput := &secretsmanager.RestoreSecretInput{
		SecretId: aws.String(secretID),
	}

	_, err := client.RestoreSecret(timeCtx, restoreSecretInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s %s\n", secretID, color.GreenString("restored"))

	return nil
}

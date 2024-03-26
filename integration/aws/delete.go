package aws

import (
	"context"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/jacbart/jaws/utils/style"
)

func ScheduleDeletion(ctx context.Context, client *secretsmanager.Client, secretID string, recoveryWindow int64) error {
	timeCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()
	deleteSecretInput := &secretsmanager.DeleteSecretInput{
		SecretId:                   aws.String(secretID),
		ForceDeleteWithoutRecovery: aws.Bool(false),
		RecoveryWindowInDays:       aws.Int64(recoveryWindow),
	}

	deleteSecretOutput, err := client.DeleteSecret(timeCtx, deleteSecretInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s set to %s on %s\n", secretID, style.FailureString("delete"), style.FailureString(deleteSecretOutput.DeletionDate.String()))
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
	fmt.Printf("%s %s\n", secretID, style.SuccessString("restored"))

	return nil
}

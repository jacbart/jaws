package secretsmanager

import (
	"context"

	"github.com/jacbart/jaws/integration/aws"
)

const (
	DELETE_IN_DAYS = 30
)

// AWSManager Delete - takes an int indicating the number of days before a secret is deleted
func (a AWSManager) Delete() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return err
	}

	l := len(a.Secrets)
	for i := 0; i < l; i++ {
		if err = aws.ScheduleDeletion(ctx, client, a.Secrets[i].ID, DELETE_IN_DAYS); err != nil {
			return err
		}
	}
	return nil
}

// AWSManager CancelDelete - cancel a secret deletion in progress
func (a AWSManager) CancelDelete() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return err
	}

	for _, secret := range a.Secrets {
		if err = aws.CancelDeletion(ctx, client, secret.ID); err != nil {
			return err
		}
	}
	return nil
}

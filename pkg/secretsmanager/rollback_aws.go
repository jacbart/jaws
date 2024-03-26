package secretsmanager

import (
	"context"

	"github.com/jacbart/jaws/integration/aws"
)

// AWSManager Rollback
func (a AWSManager) Rollback() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return err
	}

	for _, secret := range a.Secrets {
		if err = aws.RollbackSecret(ctx, client, secret.ID); err != nil {
			return err
		}
	}
	return nil
}

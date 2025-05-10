package aws

import (
	"context"

	"github.com/jacbart/jaws/integration/aws"
)

// AWS Manager Rollback
func (m Manager) Rollback() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(m, ctx)
	if err != nil {
		return err
	}

	for _, secret := range m.Secrets {
		if err = aws.RollbackSecret(ctx, client, secret.ID); err != nil {
			return err
		}
	}
	return nil
}

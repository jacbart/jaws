package aws

import (
	"context"

	"github.com/jacbart/jaws/integration/aws"
)

const (
	DELETE_IN_DAYS = 30
)

// AWS Manager Delete - takes an int indicating the number of days before a secret is deleted
func (m Manager) Delete() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := m.LoadClient(ctx)
	if err != nil {
		return err
	}

	l := len(m.Secrets)
	for i := range l {
		if err = aws.ScheduleDeletion(ctx, client, m.Secrets[i].ID, DELETE_IN_DAYS); err != nil {
			return err
		}
	}
	return nil
}

// AWS Manager CancelDelete - cancel a secret deletion in progress
func (m Manager) CancelDelete() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := m.LoadClient(ctx)
	if err != nil {
		return err
	}

	for _, secret := range m.Secrets {
		if err = aws.CancelDeletion(ctx, client, secret.ID); err != nil {
			return err
		}
	}
	return nil
}

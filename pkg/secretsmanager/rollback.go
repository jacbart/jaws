package secretsmanager

import (
	"context"
	"fmt"

	"github.com/jacbart/jaws/internal/aws"
)

// AWSManager Rollback
func (a *AWSManager) Rollback() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return err
	}

	sID, err := a.FuzzyFind(ctx)
	if err != nil {
		return fmt.Errorf("error while iterating and printing secret names: %v", err)
	}

	l := len(sID) - 1
	for i := 0; i < l; i++ {
		if err = aws.RollbackSecret(ctx, client, sID[i]); err != nil {
			return err
		}
	}
	return nil
}

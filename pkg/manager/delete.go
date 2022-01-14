package manager

import (
	"context"
	"fmt"

	"github.com/jacbart/fidelius-charm/internal/aws"
)

// AWSManager Delete
func (a *AWSManager) Delete(scheduleInDays int64) error {
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
		if err = aws.ScheduleDeletion(ctx, client, sID[i], scheduleInDays); err != nil {
			return err
		}
	}
	return nil
}

// AWSManager DeleteCancel
func (a *AWSManager) DeleteCancel(args []string) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return err
	}

	if err = aws.CancelDeletion(ctx, client, args[0]); err != nil {
		return err
	}
	return nil
}

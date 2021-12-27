package delete

import (
	"context"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/jacbart/fidelius-charm/internal/aws"
	"github.com/jacbart/fidelius-charm/utils/fzf"
)

func Delete(scheduleInDays int64) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	cfg, err := config.LoadDefaultConfig(ctx)
	if err != nil {
		return fmt.Errorf("unable to load SDK config, %v", err)
	}

	client := secretsmanager.NewFromConfig(cfg)

	sID, err := fzf.PrintListFZF(ctx, client)
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

func DeleteCancel(args []string) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	cfg, err := config.LoadDefaultConfig(ctx)
	if err != nil {
		return fmt.Errorf("unable to load SDK config, %v", err)
	}

	client := secretsmanager.NewFromConfig(cfg)

	if err = aws.CancelDeletion(ctx, client, args[0]); err != nil {
		return err
	}
	return nil
}

package rollback

import (
	"context"
	"fmt"

	"github.com/jacbart/fidelius-charm/internal/aws"
	"github.com/jacbart/fidelius-charm/utils/fzf"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
)

func Rollback() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	cfg, err := config.LoadDefaultConfig(ctx) // config.WithRegion("us-east-1"),
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
		if err = aws.RollbackSecret(ctx, client, sID[i]); err != nil {
			return err
		}
	}
	return nil
}

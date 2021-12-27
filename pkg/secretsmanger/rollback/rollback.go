package rollback

import (
	"context"
	"fmt"

	"github.com/jacbart/fidelius-charm/internal/aws"
	"github.com/jacbart/fidelius-charm/utils/fzf"
)

func Rollback() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := aws.LoadClient(ctx)
	if err != nil {
		return err
	}

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

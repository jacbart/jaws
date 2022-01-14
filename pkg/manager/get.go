package manager

import (
	"context"
	"errors"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
	"github.com/fatih/color"
)

type Secret struct {
	ID      string
	Content string
}

// AWSManager Get
func (a *AWSManager) Get(secretsIDList []string) ([]Secret, error) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	var Secrets []Secret

	var exitErr = errors.New("exit status 130")

	client, err := LoadAWSClient(a, ctx)
	if err != nil {
		return []Secret{}, err
	}

	var secretIDs []string
	if len(secretsIDList) == 0 {
		secretIDs, err = a.FuzzyFind(ctx)
		if err != nil {
			if err.Error() != exitErr.Error() {
				return []Secret{}, fmt.Errorf("iterating and printing secret names: %v", err)
			}
		}
	} else {
		secretIDs = secretsIDList
	}

	l := len(secretIDs)
	var rnfErr *types.ResourceNotFoundException

	for i := 0; i < l; i++ {
		vin := &secretsmanager.GetSecretValueInput{
			SecretId: aws.String(secretIDs[i]),
		}
		vout, err := client.GetSecretValue(ctx, vin)
		if err != nil {
			if errors.As(err, &rnfErr) {
				fmt.Printf("%s %s", color.RedString("no secret found called"), color.RedString(secretIDs[i]))
			} else {
				return []Secret{}, err
			}
		}
		Secrets = append(Secrets, Secret{
			ID:      secretIDs[i],
			Content: *vout.SecretString,
		})
	}

	return Secrets, nil
}

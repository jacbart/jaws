package aws

import (
	"context"

	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
)

func PullSecretsList(ctx context.Context, client *secretsmanager.Client, nextToken *string, filter []types.Filter) (*secretsmanager.ListSecretsOutput, error) {
	input := &secretsmanager.ListSecretsInput{
		Filters:   filter,
		NextToken: nextToken,
	}
	result, err := client.ListSecrets(ctx, input)
	if err != nil {
		return nil, err
	}
	return result, nil
}

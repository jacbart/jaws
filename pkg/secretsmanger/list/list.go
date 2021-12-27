package list

import (
	"context"
	"fmt"

	"github.com/jacbart/fidelius-charm/internal/aws"
)

func List() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	awsClient, err := aws.LoadClient(ctx)
	if err != nil {
		return err
	}

	var l int
	listSecretsOutput, err := aws.GetSecretsList(ctx, awsClient, nil)
	if err != nil {
		return err
	}
	l = len(listSecretsOutput.SecretList)
	for i := 0; i < l; i++ {
		fmt.Println(*listSecretsOutput.SecretList[i].Name)
	}
	for listSecretsOutput.NextToken != nil {
		listSecretsOutput, err = aws.GetSecretsList(ctx, awsClient, listSecretsOutput.NextToken)
		if err != nil {
			return err
		}
		l = len(listSecretsOutput.SecretList)
		for i := 0; i < l; i++ {
			fmt.Println(*listSecretsOutput.SecretList[i].Name)
		}
	}
	return nil
}

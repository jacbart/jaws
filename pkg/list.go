package fc

import (
	"context"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/jacbart/fidelius-charm/internal/aws"
)

func List() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	awsCfg, err := config.LoadDefaultConfig(ctx)
	if err != nil {
		return fmt.Errorf("unable to load AWS SDK config, %v", err)
	}

	awsClient := secretsmanager.NewFromConfig(awsCfg)

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

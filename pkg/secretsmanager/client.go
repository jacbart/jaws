package secretsmanager

import (
	"context"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
)

// LoadAWSClient
func LoadAWSClient(a *AWSManager, ctx context.Context) (*secretsmanager.Client, error) {
	var client *secretsmanager.Client

	if a.AccessID != "" {
		cfg, err := config.LoadDefaultConfig(ctx,
			config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider(a.AccessID, a.SecretKey, "")),
		)
		if err != nil {
			return nil, err
		}

		client = secretsmanager.NewFromConfig(cfg)
		return client, nil
	}
	cfg, err := config.LoadDefaultConfig(ctx)
	if err != nil {
		return nil, fmt.Errorf("unable to load AWS config, %v", err)
	}

	client = secretsmanager.NewFromConfig(cfg)

	return client, nil
}

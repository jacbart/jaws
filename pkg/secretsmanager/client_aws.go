package secretsmanager

import (
	"context"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/config"
	"github.com/aws/aws-sdk-go-v2/credentials"
	awsSM "github.com/aws/aws-sdk-go-v2/service/secretsmanager"
)

// LoadAWSClient returns a secrets manager client for aws and an error
func LoadAWSClient(a AWSManager, ctx context.Context) (*awsSM.Client, error) {
	var client *awsSM.Client
	var cfg aws.Config
	var err error

	if a.Profile != "" { // if profile is set in jaws.conf load it from the ~/.aws folder
		region := ""
		if a.Region != "" {
			region = a.Region
		} else {
			region = "us-east-1"
		}
		cfg, err = config.LoadDefaultConfig(ctx,
			config.WithSharedConfigProfile(a.Profile),
			config.WithRegion(region),
		)
		if err != nil {
			return nil, fmt.Errorf("failed loading config, %v", err)
		}
	} else if a.AccessID != "" { // if an access id is passed then load config from jaws.conf
		cfg, err = config.LoadDefaultConfig(ctx,
			config.WithCredentialsProvider(credentials.NewStaticCredentialsProvider(a.AccessID, a.SecretKey, "")),
			config.WithDefaultRegion(a.Region),
		)
	} else { // if no jaws.conf then attempt to load from boto config
		cfg, err = config.LoadDefaultConfig(ctx)
	}
	if err != nil {
		return nil, fmt.Errorf("failed loading config, %v", err)
	}
	// create secrets manager client from config
	client = awsSM.NewFromConfig(cfg)

	return client, nil
}

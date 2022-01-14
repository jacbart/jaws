package aws

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
)

func GetSecretsList(ctx context.Context, client *secretsmanager.Client, nextToken *string) (*secretsmanager.ListSecretsOutput, error) {
	input := &secretsmanager.ListSecretsInput{
		NextToken: nextToken,
	}
	result, err := client.ListSecrets(ctx, input)
	if err != nil {
		return nil, err
	}
	return result, nil
}

func GetSecretNames(secretsPath string) ([]string, error) {
	var secretNames []string
	err := filepath.WalkDir(secretsPath,
		func(path string, d os.DirEntry, err error) error {
			if err != nil {
				return err
			}
			info, err := os.Lstat(path)
			if err != nil {
				return err
			}
			if !info.IsDir() {
				secretID := strings.TrimPrefix(path, fmt.Sprintf("%s/", secretsPath))
				if !strings.HasPrefix(secretID, ".") {
					secretNames = append(secretNames, secretID)
				}
			}
			return nil
		})
	if err != nil {
		return []string{}, err
	}
	return secretNames, nil
}

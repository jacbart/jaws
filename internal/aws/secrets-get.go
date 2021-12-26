package aws

import (
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
	"github.com/fatih/color"
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

func GetSecrets(client *secretsmanager.Client, secretIDs []string, secretsPath string, cleanPrint bool, formatPrint bool) ([]string, error) {
	l := len(secretIDs)
	var rnfErr *types.ResourceNotFoundException
	var secretsList []string
	for i := 0; i < l-1; i++ {
		vin := &secretsmanager.GetSecretValueInput{
			SecretId: aws.String(secretIDs[i]),
		}
		vout, err := client.GetSecretValue(context.TODO(), vin)
		if err != nil {
			if errors.As(err, &rnfErr) {
				fmt.Printf("%s %s", color.RedString("no secret found called"), color.RedString(secretIDs[i]))
				return []string{""}, nil
			} else {
				return []string{""}, err
			}
		}
		if cleanPrint {
			cleanPrintSecret(*vout.SecretString)
		} else if formatPrint {
			formatPrintSecret(secretIDs[i], *vout.SecretString)
		} else {
			if err = downloadSecret(secretIDs[i], *vout.SecretString, secretsPath); err != nil {
				return []string{""}, err
			}
			secretsList = append(secretsList, fmt.Sprintf("%s/%s", secretsPath, secretIDs[i]))
			fmt.Printf("%s/%s\n", secretsPath, secretIDs[i])
		}
	}
	return secretsList, nil
}

func downloadSecret(secretID string, secretString string, secretsPath string) error {
	pattern := strings.Split(secretID, "/")
	filePath := fmt.Sprintf("%s/%s", secretsPath, secretID)
	dir := fmt.Sprintf("%s/%s", secretsPath, strings.Join(pattern[:len(pattern)-1], "/"))
	err := os.MkdirAll(dir, 0755)
	if err != nil {
		return err
	}
	f, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer f.Close()

	_, err = f.WriteString(secretString)
	if err != nil {
		return err
	}
	err = f.Close()
	if err != nil {
		return err
	}
	return nil
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

func formatPrintSecret(secretID string, secretString string) {
	color.Yellow(secretID)
	color.Cyan(secretString)
	fmt.Println("")
}

func cleanPrintSecret(secretString string) {
	fmt.Println(secretString)
}

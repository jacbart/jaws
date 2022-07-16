package secretsmanager

import (
	"context"
	"log"
	"sync"

	"github.com/jacbart/jaws/internal/aws"
	"github.com/ktr0731/go-fuzzyfinder"
)

func (a *AWSManager) FuzzyFind(ctx context.Context) ([]string, error) {
	var selectedIDs []string
	var allIDs []string
	go func(a *AWSManager, list *[]string) {
		ctx, cancel := context.WithCancel(context.Background())
		defer cancel()

		awsClient, err := LoadAWSClient(a, ctx)
		if err != nil {
			log.Fatalln(err)
		}

		var l int
		listSecretsOutput, err := aws.GetSecretsList(ctx, awsClient, nil)
		if err != nil {
			log.Fatalln(err)
		}
		l = len(listSecretsOutput.SecretList)
		for i := 0; i < l; i++ {
			*list = append(*list, *listSecretsOutput.SecretList[i].Name)
		}
		for listSecretsOutput.NextToken != nil {
			listSecretsOutput, err = aws.GetSecretsList(ctx, awsClient, listSecretsOutput.NextToken)
			if err != nil {
				log.Fatalln(err)
			}
			l = len(listSecretsOutput.SecretList)
			for i := 0; i < l; i++ {
				*list = append(*list, *listSecretsOutput.SecretList[i].Name)
			}
		}
	}(a, &allIDs)

	rw := sync.RWMutex{}
	l := rw.RLocker()

	idxs, _ := fuzzyfinder.FindMulti(&allIDs, func(i int) string {
		return allIDs[i]
	}, fuzzyfinder.WithHotReloadLock(l))
	for _, idx := range idxs {
		selectedIDs = append(selectedIDs, allIDs[idx])
	}
	return selectedIDs, nil
}

// AWSManager ListAll
func (a *AWSManager) ListAll() ([]string, error) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	var list []string

	awsClient, err := LoadAWSClient(a, ctx)
	if err != nil {
		return []string{}, err
	}

	var l int
	listSecretsOutput, err := aws.GetSecretsList(ctx, awsClient, nil)
	if err != nil {
		return []string{}, err
	}
	l = len(listSecretsOutput.SecretList)
	for i := 0; i < l; i++ {
		list = append(list, *listSecretsOutput.SecretList[i].Name)
	}
	for listSecretsOutput.NextToken != nil {
		listSecretsOutput, err = aws.GetSecretsList(ctx, awsClient, listSecretsOutput.NextToken)
		if err != nil {
			return []string{}, err
		}
		l = len(listSecretsOutput.SecretList)
		for i := 0; i < l; i++ {
			list = append(list, *listSecretsOutput.SecretList[i].Name)
		}
	}
	return list, nil
}

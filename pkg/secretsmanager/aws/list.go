package aws

import (
	"context"
	"errors"
	"fmt"
	"log"
	"strings"
	"sync"

	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
	"github.com/jacbart/jaws/integration/aws"
	"github.com/jacbart/jaws/utils"
	"github.com/ktr0731/go-fuzzyfinder"
)

// AWS Manager - SecretSelect takes in a slice of args and returns the secretID's to a.Secrets
func (m *Manager) SecretSelect(args []string) error {
	var secrets []Secret

	exitErr := errors.New("exit status 130")

	if len(args) > 0 {
		for _, arg := range args {
			if utils.CheckIfPrefix(arg) {
				idList := m.ListAll(strings.TrimSuffix(arg, "/*"))
				for _, id := range idList {
					secrets = append(secrets, Secret{ID: id})
				}
			} else {
				secrets = append(secrets, Secret{ID: arg})
			}
		}
	} else {
		sIds, err := m.FuzzyFind(context.Background(), "")
		if err != nil {
			if err.Error() != exitErr.Error() {
				return fmt.Errorf("iterating and printing secret names: %v", err)
			}
		}
		l := len(sIds)
		for i := range l {
			if sIds[i] != "" {
				secrets = append(secrets, Secret{ID: sIds[i]})
			}
		}
	}
	for _, s := range secrets {
		if s.ID != "" {
			m.Secrets = append(m.Secrets, s)
		}
	}
	log.Default().Println("selected secrets:", m.Secrets)
	return nil
}

// AWS Manager FuzzyFind -
func (m Manager) FuzzyFind(parentCtx context.Context, prefix string) ([]string, error) {
	var selectedIDs []string
	var allIDs []string

	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	go m.listPager(&allIDs, prefix, ctx)

	rw := sync.RWMutex{}
	l := rw.RLocker()

	idxs, _ := fuzzyfinder.FindMulti(&allIDs, func(i int) string {
		return allIDs[i]
	}, fuzzyfinder.WithHotReloadLock(l), fuzzyfinder.WithMode(fuzzyfinder.ModeCaseInsensitive))
	for _, idx := range idxs {
		selectedIDs = append(selectedIDs, allIDs[idx])
	}
	return selectedIDs, nil
}

// AWS Manager listPager - takes a pointer to a string slice, a prefix for a filter and the partent context. The list of secrets is then appended to the list pointer
func (m Manager) listPager(list *[]string, prefix string, parentCtx context.Context) {
	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	var prefixFilter []types.Filter
	if prefix != "" {
		prefix = strings.TrimSuffix(prefix, "*")
		prefixFilter = []types.Filter{
			{
				Key:    types.FilterNameStringTypeName,
				Values: []string{prefix},
			},
		}
	} else {
		prefixFilter = nil
	}
	awsClient, err := m.LoadClient(ctx)
	if err != nil {
		log.Default().Fatalln(err)
	}

	var l int
	listSecretsOutput, err := aws.PullSecretsList(ctx, awsClient, nil, prefixFilter)
	if err != nil {
		log.Default().Fatalln(err)
	}
	l = len(listSecretsOutput.SecretList)
	for i := range l {
		*list = append(*list, *listSecretsOutput.SecretList[i].Name)
	}
	for listSecretsOutput.NextToken != nil {
		listSecretsOutput, err = aws.PullSecretsList(ctx, awsClient, listSecretsOutput.NextToken, prefixFilter)
		if err != nil {
			log.Default().Fatalln(err)
		}
		l = len(listSecretsOutput.SecretList)
		for i := range l {
			*list = append(*list, *listSecretsOutput.SecretList[i].Name)
		}
	}
}

// AWS Manager ListAll - grabs and returns the entire list of secrets with an error
func (m Manager) ListAll(prefix string) []string {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	var list []string

	m.listPager(&list, prefix, ctx)
	return list
}

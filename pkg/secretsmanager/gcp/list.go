package gcp

import (
	"context"
	"errors"
	"fmt"
	"log"
	"strings"
	"sync"

	"github.com/jacbart/jaws/integration/gcp"
	"github.com/jacbart/jaws/utils"
	"github.com/ktr0731/go-fuzzyfinder"
)

// GCP Manager - SecretSelect takes in a slice of args and returns the values to g.Secrets
func (m Manager) SecretSelect(args []string) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	// load the service to find the default project
	_, err := m.LoadClient(ctx)
	if err != nil {
		return err
	}

	var secrets []Secret

	log.Default().Println("provided Args:", args)

	exitErr := errors.New("exit status 130")

	if len(args) > 0 {
		for _, arg := range args {
			if !strings.HasPrefix(arg, m.DefaultProject) {
				arg = m.DefaultProject + "/secrets/" + arg
				log.Default().Println("adding prefix:", arg)
			}
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
		sIds, err := m.FuzzyFind(ctx, "")
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

// GCPManager FuzzyFind
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

// GCP Manager listPager
func (m Manager) listPager(list *[]string, prefix string, parentCtx context.Context) {
	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	// gcp secrets service
	service, err := m.LoadClient(ctx)
	if err != nil {
		log.Default().Fatal(err)
	}

	// loop through listed projects and secrets appending them to the list
	for _, project := range m.Projects {
		// optional filter if prefix is passed
		res, err := gcp.PullSecretsList(ctx, service, prefix, project.Name, "")
		if err != nil {
			log.Default().Fatal(err)
		}
		nextToken := res.NextPageToken
		for _, secret := range res.Secrets {
			*list = append(*list, secret.Name)
		}

		for nextToken != "" {
			res, err := gcp.PullSecretsList(ctx, service, prefix, project.Name, nextToken)
			if err != nil {
				log.Default().Fatal(err)
			}
			nextToken = res.NextPageToken
			for _, secret := range res.Secrets {
				*list = append(*list, secret.Name)
			}
		}
	}
}

// GCP Manager ListAll
func (m Manager) ListAll(prefix string) []string {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	var list []string

	m.listPager(&list, prefix, ctx)
	return list
}

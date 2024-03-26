package secretsmanager

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

// GCPManager - SecretSelect takes in a slice of args and returns the values to g.Secrets
func (g *GCPManager) SecretSelect(args []string) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	// load the service to find the default project
	_, err := LoadGCPClient(g, ctx)
	if err != nil {
		return err
	}

	var secrets []Secret

	log.Default().Println("provided Args:", args)

	var exitErr = errors.New("exit status 130")

	if len(args) > 0 {
		for _, arg := range args {
			if !strings.HasPrefix(arg, g.DefaultProject) {
				arg = g.DefaultProject + "/secrets/" + arg
				log.Default().Println("adding prefix:", arg)
			}
			if utils.CheckIfPrefix(arg) {
				idList := g.ListAll(strings.TrimSuffix(arg, "/*"))
				for _, id := range idList {
					secrets = append(secrets, Secret{ID: id})
				}
			} else {
				secrets = append(secrets, Secret{ID: arg})
			}
		}
	} else {
		sIds, err := g.FuzzyFind(ctx, "")
		if err != nil {
			if err.Error() != exitErr.Error() {
				return fmt.Errorf("iterating and printing secret names: %v", err)
			}
		}
		l := len(sIds)
		for i := 0; i < l; i++ {
			if sIds[i] != "" {
				secrets = append(secrets, Secret{ID: sIds[i]})
			}
		}
	}
	for _, s := range secrets {
		if s.ID != "" {
			g.Secrets = append(g.Secrets, s)
		}
	}
	log.Default().Println("selected secrets:", g.Secrets)
	return nil
}

// GCPManager FuzzyFind
func (g GCPManager) FuzzyFind(parentCtx context.Context, prefix string) ([]string, error) {
	var selectedIDs []string
	var allIDs []string

	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	go g.listPager(&allIDs, prefix, ctx)

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

// GCPManager listPager
func (g GCPManager) listPager(list *[]string, prefix string, parentCtx context.Context) {
	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	// gcp secrets service
	service, err := LoadGCPClient(&g, ctx)
	if err != nil {
		log.Default().Fatal(err)
	}

	// loop through listed projects and secrets appending them to the list
	for _, project := range g.Projects {
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

// GCPManager ListAll
func (g GCPManager) ListAll(prefix string) []string {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()
	var list []string

	g.listPager(&list, prefix, ctx)
	return list
}

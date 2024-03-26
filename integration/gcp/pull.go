package gcp

import (
	"context"
	"fmt"
	"log"

	gcpSM "google.golang.org/api/secretmanager/v1"
)

// PullSecretsList
func PullSecretsList(pCtx context.Context, service *gcpSM.ProjectsSecretsService, prefix, project string, nextToken string) (*gcpSM.ListSecretsResponse, error) {
	var filter string
	if prefix != "" {
		filter = fmt.Sprintf("name:%s* AND state:ENABLED", prefix)
	}
	log.Default().Println(filter)
	listCall := service.List(project)
	listCall = listCall.Filter(filter)
	res, err := listCall.Do()
	if err != nil {
		return nil, err
	}

	return res, nil
}

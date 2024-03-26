package gcp

import (
	"context"
	"encoding/base64"
	"strings"
	"time"

	gcpSM "google.golang.org/api/secretmanager/v1"
)

func CheckIfUpdate(pCtx context.Context, service *gcpSM.ProjectsSecretsService, project, secretID, secretStringUpdate string) (bool, error) {
	timeCtx, cancel := context.WithTimeout(pCtx, 2*time.Second)
	defer cancel()

	secretName := project + "/secrets/" + strings.ReplaceAll(secretID, "/", "_") + "/versions/latest"

	accessCall := service.Versions.Access(secretName)

	accessCall.Context(timeCtx)
	sv, err := accessCall.Do()
	if err != nil {
		return true, nil
	}

	decodedBytes, err := base64.StdEncoding.DecodeString(sv.Payload.Data)
	if err != nil {
		return false, err
	}
	if strings.Compare(string(decodedBytes), secretStringUpdate) != 0 {
		return true, nil
	}
	return false, nil
}

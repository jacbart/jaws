package gcp

import (
	"context"
	"encoding/base64"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/jacbart/jaws/utils/style"
	gcpSM "google.golang.org/api/secretmanager/v1"
)

func secretExists(pCtx context.Context, service *gcpSM.ProjectsSecretsService, project, secretId string) bool {
	timeCtx, cancel := context.WithTimeout(pCtx, 2*time.Second)
	defer cancel()

	secretName := project + "/secrets/" + strings.ReplaceAll(secretId, "/", "_")
	getCall := service.Get(secretName)

	getCall.Context(timeCtx)
	_, err := getCall.Do()
	log.Default().Println(err)
	return err == nil
}

func createSecret(pCtx context.Context, service *gcpSM.ProjectsSecretsService, project, secretId string) error {
	timeCtx, cancel := context.WithTimeout(pCtx, 2*time.Second)
	defer cancel()

	labels := make(map[string]string)
	labels["managed-by"] = "jaws"

	gcpSecret := &gcpSM.Secret{
		Replication: &gcpSM.Replication{
			Automatic: &gcpSM.Automatic{},
		},
		Labels: labels,
	}
	secretId = strings.ReplaceAll(secretId, "/", "_")

	createCall := service.Create(project, gcpSecret)

	createCall.Context(timeCtx)
	createCall.SecretId(secretId)

	_, err := createCall.Do()
	if err != nil {
		if !strings.Contains(err.Error(), "409") {
			return err
		}
	}
	return nil
}

func AddSecretVersion(pCtx context.Context, service *gcpSM.ProjectsSecretsService, project, secretId, secretString string) error {
	ctx, cancel := context.WithCancel(pCtx)
	defer cancel()

	encoded := base64.StdEncoding.EncodeToString([]byte(secretString))
	payload := &gcpSM.SecretPayload{
		Data: encoded,
	}
	addVersionRequest := &gcpSM.AddSecretVersionRequest{
		Payload: payload,
	}

	addVersionCall := service.AddVersion(project+"/secrets/"+secretId, addVersionRequest)

	addVersionCall.Context(ctx)
	_, err := addVersionCall.Do()
	if err != nil {
		return err
	}
	return nil
}

func HandleUpdateCreate(ctx context.Context, service *gcpSM.ProjectsSecretsService, project, secretId, secretString string, createPrompt bool) error {
	var userResponse string
	log.Default().Println(secretId)
	if secretExists(ctx, service, project, secretId) {
		// addsecretversion
		if err := AddSecretVersion(ctx, service, project, secretId, secretString); err != nil {
			return err
		}
		fmt.Printf("%s %s\n", project+"/secrets/"+secretId, style.ChangedString("updated"))
	} else {
		if !createPrompt {
			fmt.Printf("%s was not found, would you like to create this secret? [y/N] ", secretId)
			fmt.Scanln(&userResponse)

			userResponse = strings.TrimSpace(userResponse)
			userResponse = strings.ToLower(userResponse)

			if userResponse == "y" || userResponse == "yes" {
				// createsecret
				if err := createSecret(ctx, service, project, secretId); err != nil {
					return err
				}
				// addsecretversion
				if err := AddSecretVersion(ctx, service, project, secretId, secretString); err != nil {
					return err
				}
				fmt.Printf("%s %s\n", secretId, style.ChangedString("created"))
			} else {
				fmt.Printf("creation of %s %s\n", secretId, style.InfoString("skipped"))
			}
		} else {
			// createsecret
			if err := createSecret(ctx, service, project, secretId); err != nil {
				return err
			}
			// addsecretversion
			if err := AddSecretVersion(ctx, service, project, secretId, secretString); err != nil {
				return err
			}
		}
	}
	return nil
}

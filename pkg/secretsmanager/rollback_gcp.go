package secretsmanager

import (
	"context"
	"encoding/base64"
	"log"
	"strings"

	"github.com/jacbart/jaws/integration/gcp"
	"github.com/jacbart/jaws/utils/style"
	"github.com/jacbart/jaws/utils/tui"
	gcpSM "google.golang.org/api/secretmanager/v1"
)

// GCPManager Rollback
func (g GCPManager) Rollback() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	service, err := LoadGCPClient(&g, ctx)
	if err != nil {
		return err
	}

	for _, secret := range g.Secrets {
		versions := gcpVersionList(ctx, service, secret.ID)
		versionSel, err := tui.SelectorTUI(versions)
		if err != nil {
			return err
		}
		log.Default().Println(style.InfoString(versionSel), style.InfoString("Selected"))
		// get selected versions payload
		accessVersionCall := service.Versions.Access(versionSel)
		accessVersionCall.Context(ctx)
		res, err := accessVersionCall.Do()
		if err != nil {
			return err
		}
		decodedBytes, err := base64.StdEncoding.DecodeString(res.Payload.Data)
		if err != nil {
			return err
		}
		// push as an updated version
		err = gcp.AddSecretVersion(ctx, service, g.DefaultProject, strings.TrimPrefix(secret.ID, g.DefaultProject+"/secrets/"), string(decodedBytes))
		if err != nil {
			return err
		}
	}
	return nil
}

func gcpVersionList(parentCtx context.Context, service *gcpSM.ProjectsSecretsService, secretId string) []string {
	log.Default().Println(secretId)
	var versions []string
	versionsCall := service.Versions.List(secretId)
	pagerToken := "0"
	for {
		if pagerToken == "" {
			break
		} else if pagerToken != "0" {
			versionsCall.PageToken(pagerToken)
		}
		versionsCall.Context(parentCtx)
		res, err := versionsCall.Do()
		if err != nil {
			return nil
		}
		pagerToken = res.NextPageToken

		for _, v := range res.Versions {
			log.Default().Println(v.Name)
			versions = append(versions, v.Name)
		}
	}
	return versions
}

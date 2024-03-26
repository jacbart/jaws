package secretsmanager

import (
	"context"
	"errors"
	"fmt"
	"os"
	"strings"

	"github.com/jacbart/jaws/utils/style"
	"github.com/jacbart/jaws/utils/tui"
	"google.golang.org/api/cloudresourcemanager/v3"
	"google.golang.org/api/option"
	gcpSM "google.golang.org/api/secretmanager/v1"
)

// LoadGCPClient returns a GCP service client
func LoadGCPClient(g *GCPManager, ctx context.Context) (*gcpSM.ProjectsSecretsService, error) {
	var secretService *gcpSM.Service
	var secretClient *gcpSM.ProjectsSecretsService
	var projService *cloudresourcemanager.Service
	var err error
	if g.CredFile != "" { // use creds file
		secretService, err = gcpSM.NewService(ctx, option.WithCredentialsFile(g.CredFile))
		if err != nil {
			return nil, err
		}

		projService, err = cloudresourcemanager.NewService(ctx, option.WithCredentialsFile(g.CredFile))
		if err != nil {
			return nil, err
		}
	} else if g.APIKey != "" { // use API key
		secretService, err = gcpSM.NewService(ctx, option.WithAPIKey(g.APIKey))
		if err != nil {
			return nil, err
		}

		projService, err = cloudresourcemanager.NewService(ctx, option.WithAPIKey(g.APIKey))
		if err != nil {
			return nil, err
		}
	} else { // default creds
		secretService, err = gcpSM.NewService(ctx)
		if err != nil {
			return nil, err
		}

		projService, err = cloudresourcemanager.NewService(ctx)
		if err != nil {
			return nil, err
		}

	}
	// get list of available projects and add to the GCPManager
	if g.Projects == nil {
		err = g.getProjects(projService, ctx)
		if err != nil {
			return nil, err
		}
	}
	secretClient = gcpSM.NewProjectsSecretsService(secretService)
	return secretClient, nil
}

// GCPManager getProjects lists out all available projects the user/service account has access to
func (g *GCPManager) getProjects(service *cloudresourcemanager.Service, ctx context.Context) error {
	var projs []*cloudresourcemanager.Project
	projService := cloudresourcemanager.NewProjectsService(service)

	// call gcp and get list of projects for account
	searchCall := projService.Search()
	res, err := searchCall.Do()
	if err != nil {
		return err
	}
	projs = append(projs, res.Projects...)

	// continue looping till all projects are appended to projs
	for {
		if res.NextPageToken == "" {
			break
		}

		searchCall.PageToken(res.NextPageToken)
		res, err = searchCall.Do()
		if err != nil {
			return err
		}
		projs = append(projs, res.Projects...)
	}

	l := len(projs)
	// if more than one project then have the user choose one, optionally pass the default to skip this
	if l <= 0 {
		return errors.New("account has no projects to access")
	} else if l == 1 {
		if g.DefaultProject != projs[0].Name && g.DefaultProject != "" {
			var userResponse string
			fmt.Printf("gcp default project is not found or is unavailable\ncontinue with %s? [y/N] ", projs[0].Name)
			fmt.Scanln(&userResponse)

			userResponse = strings.TrimSpace(userResponse)
			userResponse = strings.ToLower(userResponse)

			if userResponse == "y" || userResponse == "yes" {
				g.DefaultProject = projs[0].Name
				fmt.Println("gcp project:", style.SuccessString(g.DefaultProject))
			} else {
				fmt.Println("quitting...")
				os.Exit(0)
			}
		}
		if g.DefaultProject == "" {
			g.DefaultProject = projs[0].Name
		}
	} else {
		var projNames []string
		for _, proj := range projs {
			projNames = append(projNames, proj.Name)
		}
		g.DefaultProject, err = tui.SelectorTUI(projNames)
		if err != nil {
			return err
		}
		fmt.Println("gcp project:", style.SuccessString(g.DefaultProject))
	}

	g.Projects = append(g.Projects, projs...)
	return nil
}

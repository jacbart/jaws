package gcp

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

// GCP Manager LoadClient returns a GCP service client
func (m Manager) LoadClient(ctx context.Context) (*gcpSM.ProjectsSecretsService, error) {
	var secretService *gcpSM.Service
	var secretClient *gcpSM.ProjectsSecretsService
	var projService *cloudresourcemanager.Service
	var err error
	if m.CredFile != "" { // use creds file
		secretService, err = gcpSM.NewService(ctx, option.WithCredentialsFile(m.CredFile))
		if err != nil {
			return nil, err
		}

		projService, err = cloudresourcemanager.NewService(ctx, option.WithCredentialsFile(m.CredFile))
		if err != nil {
			return nil, err
		}
	} else if m.APIKey != "" { // use API key
		secretService, err = gcpSM.NewService(ctx, option.WithAPIKey(m.APIKey))
		if err != nil {
			return nil, err
		}

		projService, err = cloudresourcemanager.NewService(ctx, option.WithAPIKey(m.APIKey))
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
	if m.Projects == nil {
		err = m.getProjects(projService)
		if err != nil {
			return nil, err
		}
	}
	secretClient = gcpSM.NewProjectsSecretsService(secretService)
	return secretClient, nil
}

// GCPManager getProjects lists out all available projects the user/service account has access to
func (m Manager) getProjects(service *cloudresourcemanager.Service) error {
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
	for res.NextPageToken != "" {
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
		if m.DefaultProject != projs[0].Name && m.DefaultProject != "" {
			var userResponse string
			fmt.Printf("gcp default project is not found or is unavailable\ncontinue with %s? [y/N] ", projs[0].Name)
			fmt.Scanln(&userResponse)

			userResponse = strings.TrimSpace(userResponse)
			userResponse = strings.ToLower(userResponse)

			if userResponse == "y" || userResponse == "yes" {
				m.DefaultProject = projs[0].Name
				fmt.Println("gcp project:", style.SuccessString(m.DefaultProject))
			} else {
				fmt.Println("quitting...")
				os.Exit(0)
			}
		}
		if m.DefaultProject == "" {
			m.DefaultProject = projs[0].Name
		}
	} else {
		var projNames []string
		for _, proj := range projs {
			projNames = append(projNames, proj.Name)
		}
		m.DefaultProject, err = tui.SelectorTUI(projNames)
		if err != nil {
			return err
		}
		fmt.Println("gcp project:", style.SuccessString(m.DefaultProject))
	}

	m.Projects = append(m.Projects, projs...)
	return nil
}

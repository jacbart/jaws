package gcp

import (
	"google.golang.org/api/cloudresourcemanager/v3"
)

// Manager
type Manager struct {
	Secrets        []Secret
	ProfileLabel   string
	Projects       []*cloudresourcemanager.Project
	DefaultProject string
	CredFile       string `hcl:"creds_file,optional"`
	APIKey         string `hcl:"api_key,optional"`
}

// GCPManager ProfileName returns the name of the default profile
func (m Manager) ProfileName() string {
	return m.ProfileLabel
}

// GCPManager Platform returns aws
func (m Manager) Platform() string {
	return "gcp"
}

// GCPManager Region returns aws
func (m Manager) Locale() string {
	return ""
}

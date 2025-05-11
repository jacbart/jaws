package bws

import (
	sm "github.com/jacbart/jaws/pkg/secretsmanager"
)

// BWS Manager
type Manager struct {
	Secrets      []sm.Secret
	ProfileLabel string
	OrgID        string `hcl:"org_id,optional"`
	StateFile    string `hcl:"state_file,optional"`
	AccessToken  string `hcl:"access_token,optional"`
}

// BWS Manager ProfileName returns the name of the profile
func (m Manager) ProfileName() string {
	return m.ProfileLabel
}

// BWS Manager Platform returns bws
func (m Manager) Platform() string {
	return "bws"
}

// BWS Manager Locale returns nothing
func (m Manager) Locale() string {
	return ""
}

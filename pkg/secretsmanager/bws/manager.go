package bws

import (
	sm "github.com/jacbart/jaws/pkg/secretsmanager"
)

// Manager
type Manager struct {
	Secrets      []sm.Secret
	ProfileLabel string
	StateFile    string `hcl:"state_file,optional"`
	AccessToken  string `hcl:"access_token,optional"`
}

// BWSManager ProfileName returns the name of the profile
func (m Manager) ProfileName() string {
	return m.ProfileLabel
}

// BWSManager Platform returns bws
func (m Manager) Platform() string {
	return "bws"
}

// BWSManager Locale returns nothing
func (m Manager) Locale() string {
	return ""
}

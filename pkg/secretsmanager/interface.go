package secretsmanager

import (
	"context"

	"google.golang.org/api/cloudresourcemanager/v3"
)

// Manager interface
type Manager interface {
	ProfileName() string
	Platform() string
	Locale() string
	Delete() error
	CancelDelete() error
	FuzzyFind(context.Context, string) ([]string, error)
	SecretSelect(args []string) error
	Pull(prefix string) ([]Secret, error)
	ListAll(string) []string
	Rollback() error
	Push(string, bool) error
}

// Secret holds the ID and content of a secret
type Secret struct {
	ID      string
	Content string
}

// CliConfig
type CliConfig struct {
	Conf          ConfigHCL
	FileName      string
	FilePaths     []string
	CurrentConfig string
	Key           string
}

// AWSManager
type AWSManager struct {
	Secrets      []Secret
	ProfileLabel string
	Profile      string `hcl:"profile,optional"`
	AccessID     string `hcl:"access_id,optional"`
	SecretKey    string `hcl:"secret_key,optional"`
	Region       string `hcl:"region,optional"`
}

// GCPManager
type GCPManager struct {
	Secrets        []Secret
	ProfileLabel   string
	Projects       []*cloudresourcemanager.Project
	DefaultProject string
	CredFile       string `hcl:"creds_file,optional"`
	APIKey         string `hcl:"api_key,optional"`
}

// BWSManager
type BWSManager struct {
	Secrets      []Secret
	ProfileLabel string
	StateFile    string `hcl:"state_file,optional"`
	AccessToken  string `hcl:"access_token,optional"`
}

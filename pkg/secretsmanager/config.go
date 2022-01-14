package secretsmanager

import (
	"context"

	"github.com/hashicorp/hcl/v2"
)

type Manager interface {
	ProfileName() string
	Create([]string, string, bool) error
	Delete(int64) error
	DeleteCancel([]string) error
	FuzzyFind(context.Context) ([]string, error)
	Get([]string) ([]Secret, error)
	ListAll() ([]string, error)
	Rollback() error
	Set(string, bool) error
}

type Config struct {
	General  *GeneralHCL   `hcl:"general,block"`
	Managers []*managerHCL `hcl:"manager,block"`
}

type GeneralHCL struct {
	DefaultProfile string `hcl:"default_profile,optional"`
	Editor         string `hcl:"editor,optional"`
	SecretsPath    string `hcl:"secrets_path,optional"`
}

type managerHCL struct {
	Platform string   `hcl:"platform,label"`
	Profile  string   `hcl:"profile,label"`
	Auth     hcl.Body `hcl:",remain"`
}

type AWSManager struct {
	Profile   string
	AccessID  string `hcl:"access_id,optional"`
	SecretKey string `hcl:"secret_key,optional"`
	Region    string `hcl:"region,optional"`
}

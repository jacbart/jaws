package config

import (
	_ "embed"
	"fmt"
	"io"
	"os"

	"github.com/hashicorp/hcl/v2"
	"github.com/hashicorp/hcl/v2/gohcl"
	"github.com/hashicorp/hcl/v2/hclparse"
	"github.com/jacbart/jaws/pkg/lockandload"
	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/pkg/secretsmanager/aws"
	"github.com/jacbart/jaws/pkg/secretsmanager/gcp"
	"github.com/zclconf/go-cty/cty/function"
)

// ConfigHCL
type ConfigHCL struct {
	General  GeneralHCL   `hcl:"general,block"`
	Managers []managerHCL `hcl:"manager,block"`
}

// GeneralHCL
type GeneralHCL struct {
	DefaultProfile         string `hcl:"default_profile,optional"`
	DisableDetectJawsFiles bool   `hcl:"disable_auto_detect,optional"`
	SafeMode               bool   `hcl:"safe_mode,optional"`
	RepoWarn               bool   `hcl:"repo_warn,optional"`
	Editor                 string `hcl:"editor,optional"`
	SecretsPath            string `hcl:"secrets_path,optional"`
	GithubToken            string `hcl:"gh_token,optional"`
}

// managerHCL
type managerHCL struct {
	Platform     string   `hcl:"platform,label"`
	ProfileLabel string   `hcl:",label"`
	Auth         hcl.Body `hcl:",remain"`
}

// CliConfig
type CliConfig struct {
	Conf          ConfigHCL
	FileName      string
	FilePaths     []string
	CurrentConfig string
	Key           string
}

// InitCliConfig
func InitCliConfig() CliConfig {
	return CliConfig{}
}

// SetConfigName
func (c *CliConfig) SetConfigName(file string) {
	c.FileName = file
}

// AddConfigPath
func (c *CliConfig) AddConfigPath(path string) {
	c.FilePaths = append(c.FilePaths, path)
}

// ReadInConfig
func (c *CliConfig) ReadInConfig() ([]secretsmanager.Manager, error) {
	err := checkForConfig(c)
	if err != nil {
		return nil, err
	}

	// Load config file using lockandload
	f, err := lockandload.NewSecureFile(c.CurrentConfig, os.Getenv("JAWS_CONFIG_KEY"))
	if err != nil {
		return nil, err
	}
	input, err := f.Load()
	if err != nil {
		return nil, err
	}
	c.Key = f.Key

	src, err := io.ReadAll(input)
	if err != nil {
		return nil, fmt.Errorf(
			"error in ReadInConfig reading input `%s`: %w", c.CurrentConfig, err,
		)
	}

	parser := hclparse.NewParser()
	srcHCL, diag := parser.ParseHCL(src, c.CurrentConfig)
	if diag.HasErrors() {
		return nil, fmt.Errorf(
			"error in ReadInConfig parsing HCL: %w", diag,
		)
	}

	evalContext, err := createContext()
	if err != nil {
		return nil, fmt.Errorf(
			"error in ReadInConfig creating HCL evaluation context: %w", err,
		)
	}

	configHCL := &ConfigHCL{}
	if diag := gohcl.DecodeBody(srcHCL.Body, evalContext, configHCL); diag.HasErrors() {
		return nil, &DecodeConfigFailed{File: c.CurrentConfig}
	}

	managers := []secretsmanager.Manager{}
	for _, m := range configHCL.Managers {
		switch managerPlatform := m.Platform; managerPlatform {
		case "aws":
			manager := &aws.Manager{ProfileLabel: m.ProfileLabel}
			if m.Auth != nil {
				if diag := gohcl.DecodeBody(m.Auth, evalContext, manager); diag.HasErrors() {
					return nil, &DecodeConfigFailed{File: c.CurrentConfig}
				}
			}
			managers = append(managers, manager)
		case "gcp":
			manager := &gcp.Manager{ProfileLabel: m.ProfileLabel}
			if m.Auth != nil {
				if diag := gohcl.DecodeBody(m.Auth, evalContext, manager); diag.HasErrors() {
					return nil, &DecodeConfigFailed{File: c.CurrentConfig}
				}
			}
			managers = append(managers, manager)
		default:
			return nil, fmt.Errorf("error in ReadInConfig: unknown platform `%s`", managerPlatform)
		}
	}
	c.Conf.General = configHCL.General
	c.Conf.Managers = configHCL.Managers
	return managers, nil
}

// checkForConfig
func checkForConfig(c *CliConfig) error {
	if len(c.FilePaths) == 0 {
		fs, err := os.Stat(c.FileName)
		if err == nil {
			if fs.Size() == 0 {
				return &NoConfigFileFound{c.FileName, []string{"."}}
			}
			c.CurrentConfig = c.FileName
			return nil
		} else {
			return &NoConfigFileFound{c.FileName, []string{"."}}
		}
	}
	for _, path := range c.FilePaths {
		fs, err := os.Stat(fmt.Sprintf("%s/%s", path, c.FileName))
		if err == nil {
			if fs.Size() == 0 {
				continue
			}
			c.CurrentConfig = fmt.Sprintf("%s/%s", path, c.FileName)
			return nil
		}
	}
	return &NoConfigFileFound{c.FileName, c.FilePaths}
}

// createContext
func createContext() (*hcl.EvalContext, error) {
	functions := map[string]function.Function{}

	// Return the constructed hcl.EvalContext.
	return &hcl.EvalContext{
		Functions: functions,
	}, nil
}

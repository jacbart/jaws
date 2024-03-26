package secretsmanager

import (
	_ "embed"
	"fmt"
	"io"
	"log"
	"os"
	"strconv"
	"text/template"

	"github.com/hashicorp/hcl/v2"
	"github.com/hashicorp/hcl/v2/gohcl"
	"github.com/hashicorp/hcl/v2/hclparse"
	"github.com/jacbart/jaws/pkg/lockandload"
	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/tui"
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

//go:embed config.tmpl
var configTmpl string

// CreateConfig outputs a simple valid jaws.config to stdout
func CreateConfig(conf *ConfigHCL) error {
	var c ConfigHCL
	var tmplType string
	if conf != nil {
		tmplType = "empty"
		c = *conf
	} else {
		tmplType = "cicd"
		c = ConfigHCL{
			General: GeneralHCL{
				DefaultProfile:         "default",
				DisableDetectJawsFiles: false,
				Editor:                 os.Getenv("EDITOR"),
				GithubToken:            os.Getenv("GH_TOKEN"),
				RepoWarn:               true,
				SafeMode:               true,
				SecretsPath:            fmt.Sprintf("%s/.jaws/secrets", os.Getenv("HOME")),
			},
			Managers: []managerHCL{
				{
					Platform:     "aws",
					ProfileLabel: "default",
					Auth:         nil,
				},
			},
		}
	}

	tmpl, err := template.New("jaws.conf").Funcs(utils.TemplateFuncs).Parse(configTmpl)
	if err != nil {
		return fmt.Errorf("tmpl parse phase: %w", err)
	}
	err = tmpl.ExecuteTemplate(os.Stdout, tmplType, c)
	if err != nil {
		return fmt.Errorf("tmpl execution phase: %w", err)
	}
	return nil
}

// SetupWizard prompts user to input and returns a ConfigHCL and error
func SetupWizard() (ConfigHCL, error) {
	inputModel := []tui.ModelVars{
		{
			Description: "secrets_path        | secrets download folder",
			Placeholder: os.Getenv("HOME") + "/.jaws/secrets",
			Width:       64,
		},
		{
			Description: "editor              | the editor used if '-e' is passed",
			Placeholder: os.Getenv("EDITOR"),
			Width:       25,
		},
		{
			Description: "gh_token            | github token with access to the jaws repo used for the update command",
			Placeholder: os.Getenv("GH_TOKEN"),
			Width:       124,
		},
		{
			Description: "disable_auto_detect | false enables scanning current directory for '.jaws' files on the pull command",
			Placeholder: "false",
			Width:       5,
		},
		{
			Description: "repo_warn           | true enables a warning message if using jaws in a git repo",
			Placeholder: "true",
			Width:       5,
		},
		{
			Description: "safe_mode           | true disables overwrite prompts and instead moves and dates old env files",
			Placeholder: "false",
			Width:       5,
		},
	}

	results, err := tui.InputTUI(inputModel)
	if err != nil {
		return ConfigHCL{}, err
	}

	log.Default().Printf("secretsmanager: config input results\n%s\n", results)

	resultThree, err := strconv.ParseBool(results[3])
	if err != nil {
		return ConfigHCL{}, err
	}
	resultFour, err := strconv.ParseBool(results[4])
	if err != nil {
		return ConfigHCL{}, err
	}
	resultFive, err := strconv.ParseBool(results[5])
	if err != nil {
		return ConfigHCL{}, err
	}

	c := ConfigHCL{
		General: GeneralHCL{
			DefaultProfile:         "default",
			DisableDetectJawsFiles: resultThree,
			Editor:                 results[1],
			GithubToken:            results[2],
			RepoWarn:               resultFour,
			SafeMode:               resultFive,
			SecretsPath:            results[0],
		},
		Managers: []managerHCL{
			{
				Platform:     "aws",
				ProfileLabel: "default",
				Auth:         nil,
			},
		},
	}

	return c, nil
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
func (c *CliConfig) ReadInConfig() ([]Manager, error) {
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

	managers := []Manager{}
	for _, m := range configHCL.Managers {
		switch managerPlatform := m.Platform; managerPlatform {
		case "aws":
			aws := &AWSManager{ProfileLabel: m.ProfileLabel}
			if m.Auth != nil {
				if diag := gohcl.DecodeBody(m.Auth, evalContext, aws); diag.HasErrors() {
					return nil, &DecodeConfigFailed{File: c.CurrentConfig}
				}
			}
			managers = append(managers, aws)
		case "gcp":
			gcp := &GCPManager{ProfileLabel: m.ProfileLabel}
			if m.Auth != nil {
				if diag := gohcl.DecodeBody(m.Auth, evalContext, gcp); diag.HasErrors() {
					return nil, &DecodeConfigFailed{File: c.CurrentConfig}
				}
			}
			managers = append(managers, gcp)
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

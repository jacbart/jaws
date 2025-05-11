package config

import (
	_ "embed"
	"fmt"
	"log"
	"os"
	"strconv"
	"text/template"

	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/tui"
)

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

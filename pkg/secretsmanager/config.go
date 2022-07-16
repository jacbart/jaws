package secretsmanager

import (
	"bufio"
	"context"
	_ "embed"
	"fmt"
	"log"
	"os"
	"text/template"

	"github.com/hashicorp/hcl/v2"
	"github.com/jacbart/jaws/utils/helpers"
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
	General  GeneralHCL   `hcl:"general,block"`
	Managers []managerHCL `hcl:"manager,block"`
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

//go:embed config.tmpl
var configTmpl string

func CreateConfig() error {
	c := Config{
		General: GeneralHCL{
			DefaultProfile: "default",
			Editor:         os.Getenv("EDITOR"),
			SecretsPath:    fmt.Sprintf("%s/.jaws/secrets", os.Getenv("HOME")),
		},
		Managers: []managerHCL{
			{
				Platform: "aws",
				Profile:  "default",
				Auth:     nil,
			},
		},
	}

	tmpl, err := template.New("jaws.config").Funcs(helpers.TemplateFuncs).Parse(configTmpl)
	if err != nil {
    return fmt.Errorf("tmpl parse phase: %w", err)
	}
	err = tmpl.Execute(os.Stdout, c)
	if err != nil {
		return fmt.Errorf("tmpl execution phase: %w", err)
	}
	return nil
}

func ShowConfig(path string) error {
	file, err := os.Open(path)
	if err != nil {
		return err
	}
	defer func() {
		if err = file.Close(); err != nil {
			log.Fatal(err)
		}
	}()

	scanner := bufio.NewScanner(file)

	for scanner.Scan() {
		fmt.Println(scanner.Text())
	}
	return nil
}

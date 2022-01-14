package manager

import (
	"fmt"
	"io/ioutil"
	"os"
	"strings"

	"github.com/hashicorp/hcl/v2"
	"github.com/hashicorp/hcl/v2/gohcl"
	"github.com/hashicorp/hcl/v2/hclparse"
	"github.com/zclconf/go-cty/cty"
	"github.com/zclconf/go-cty/cty/function"
)

const (
	environmentKey = "env"
	envVarPrefix   = "FC_"
)

type FirmConfig struct {
	Conf      Config
	FileName  string
	FilePaths []string
}

// NewFirmConfig
func NewFirmConfig() FirmConfig {
	return FirmConfig{}
}

// SetConfigName
func (c *FirmConfig) SetConfigName(file string) {
	c.FileName = file
}

// AddConfigPath
func (c *FirmConfig) AddConfigPath(path string) {
	c.FilePaths = append(c.FilePaths, path)
}

// ReadInConfig
func (c *FirmConfig) ReadInConfig() (*GeneralHCL, []Manager, error) {
	f, err := checkForConfig(c)
	if err != nil {
		return nil, nil, err
	}
	input, err := os.Open(f)
	if err != nil {
		return nil, nil, fmt.Errorf(
			"error in ReadConfig opening config file: %w", err,
		)
	}
	defer input.Close()

	src, err := ioutil.ReadAll(input)
	if err != nil {
		return nil, nil, fmt.Errorf(
			"error in ReadConfig reading input `%s`: %w", f, err,
		)
	}

	parser := hclparse.NewParser()
	srcHCL, diag := parser.ParseHCL(src, f)
	if diag.HasErrors() {
		return nil, nil, fmt.Errorf(
			"error in ReadConfig parsing HCL: %w", diag,
		)
	}

	evalContext, err := createContext()
	if err != nil {
		return nil, nil, fmt.Errorf(
			"error in ReadConfig creating HCL evaluation context: %w", err,
		)
	}

	configHCL := &Config{}
	if diag := gohcl.DecodeBody(srcHCL.Body, evalContext, configHCL); diag.HasErrors() {
		return nil, nil, fmt.Errorf(
			"error in ReadConfig decoding HCL configuration: %w", diag,
		)
	}

	managers := []Manager{}
	for _, c := range configHCL.Managers {
		switch managerPlatform := c.Platform; managerPlatform {
		case "aws":
			aws := &AWSManager{Profile: c.Profile}
			if c.Auth != nil {
				if diag := gohcl.DecodeBody(c.Auth, evalContext, aws); diag.HasErrors() {
					return nil, nil, fmt.Errorf(
						"error in ReadConfig decoding aws HCL configuration: %w", diag,
					)
				}
			}
			managers = append(managers, aws)
		default:
			return nil, nil, fmt.Errorf("error in ReadConfig: unknown platform `%s`", managerPlatform)
		}
	}
	return configHCL.General, managers, nil
}

// checkForConfig
func checkForConfig(c *FirmConfig) (string, error) {
	if len(c.FilePaths) == 0 {
		if _, err := os.Stat(c.FileName); err == nil {
			return c.FileName, nil
		} else {
			return "", &NoConfigFileFound{c.FileName, []string{"."}}
		}
	}
	for _, path := range c.FilePaths {
		if _, err := os.Stat(fmt.Sprintf("%s/%s", path, c.FileName)); err == nil {
			return fmt.Sprintf("%s/%s", path, c.FileName), nil
		}
	}
	return "", &NoConfigFileFound{c.FileName, c.FilePaths}
}

// createContext
func createContext() (*hcl.EvalContext, error) {
	prefixed := map[string]cty.Value{}
	for _, e := range os.Environ() {
		e := strings.SplitN(e, "=", 2)
		if len(e) != 2 {
			continue
		}
		key := e[0]
		value := e[1]

		if strings.HasPrefix(key, envVarPrefix) {
			key := strings.TrimPrefix(key, envVarPrefix)
			prefixed[key] = cty.StringVal(value)
		}
	}

	// variables is a list of cty.Value for use in Decoding HCL. These will
	// be nested by using ObjectVal as a value.
	variables := map[string]cty.Value{
		environmentKey: cty.ObjectVal(prefixed),
	}

	functions := map[string]function.Function{}

	// Return the constructed hcl.EvalContext.
	return &hcl.EvalContext{
		Variables: variables,
		Functions: functions,
	}, nil
}

package envmanager

import (
	"io"

	"github.com/hashicorp/hcl/v2"
	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/zclconf/go-cty/cty"
)

const (
	SECRET_KEY        = "secret"
	LOCALS_KEY        = "var"
	ENV_PREFIX        = "JAWS_"
	FILE_FUNC_SUCCESS = "DataWrittenToFileSuccessfully"
)

type EnvConfig struct {
	Env       []*EnvHCL
	SecretIDs []string
	Options   Options
}

type EnvHCL struct {
	Prepared   bool
	Processed  bool
	Reader     io.Reader
	ConfigFile string
	Message    string   `hcl:"msg,optional"`
	OutFile    string   `hcl:"out,optional"`
	OutFormat  string   `hcl:"format,optional"`
	Profile    string   `hcl:"profile,optional"`
	Filter     string   `hcl:"filter,optional"`
	Includes   []string `hcl:"include,optional"`

	Locals []*struct {
		TmplVars hcl.Attributes `hcl:",remain"`
	} `hcl:"locals,block"`

	GroupedVars []*struct {
		TmplVars hcl.Attributes `hcl:",remain"`
	} `hcl:"vars,block"`

	GroupedLabeledVars []*struct {
		Label    string         `hcl:",label"`
		TmplVars hcl.Attributes `hcl:",remain"`
	} `hcl:"group,block"`
}

type Options struct {
	Diff           bool
	Overwrite      bool
	UnsafeMode     bool
	FilterOverride string
}

// InitEnv - initalizes EnvConfig
func InitEnv(opts *Options) EnvConfig {
	if opts == nil {
		opts = &Options{
			Diff:           false,
			Overwrite:      false,
			UnsafeMode:     false,
			FilterOverride: "",
		}
	}
	return EnvConfig{
		Options: *opts,
	}
}

// createEnvHCLContext
func createEnvHCLContext(e *EnvHCL, srcHCL *hcl.File, secrets []secretsmanager.Secret, prefixes []string) (*hcl.EvalContext, error) {
	envVars := decodeEnvVars()
	localVars, diag := decodeLocalVars(e, srcHCL, envVars)
	if diag.HasErrors() {
		return nil, diag
	}

	// append/overwrite the environment variables to the locals map
	for key, val := range envVars {
		localVars[key] = val
	}

	secretsVars, diag := decodeSecretVars(e, srcHCL, localVars, secrets, prefixes)
	if diag.HasErrors() {
		return nil, diag
	}

	// variables is a list of cty.Value for use in Decoding HCL. These will
	// be nested by using ObjectVal as a value.
	variables := map[string]cty.Value{
		LOCALS_KEY: cty.ObjectVal(localVars),
		SECRET_KEY: cty.ObjectVal(secretsVars),
	}

	functions := contextFuncs()

	// Return the constructed hcl.EvalContext.
	return &hcl.EvalContext{
		Variables: variables,
		Functions: functions,
	}, nil
}

// createLocalContext
func createLocalContext(envVars map[string]cty.Value) (*hcl.EvalContext, error) {
	// variables is a list of cty.Value for use in Decoding HCL. These will
	// be nested by using ObjectVal as a value.
	variables := map[string]cty.Value{
		LOCALS_KEY: cty.ObjectVal(envVars),
	}

	functions := contextFuncs()

	// Return the constructed hcl.EvalContext.
	return &hcl.EvalContext{
		Variables: variables,
		Functions: functions,
	}, nil
}

// createSecretContext
func createSecretContext(localVars map[string]cty.Value) (*hcl.EvalContext, error) {
	// variables is a list of cty.Value for use in Decoding HCL. These will
	// be nested by using ObjectVal as a value.
	variables := map[string]cty.Value{
		LOCALS_KEY: cty.ObjectVal(localVars),
	}

	functions := contextFuncs()

	// Return the constructed hcl.EvalContext.
	return &hcl.EvalContext{
		Variables: variables,
		Functions: functions,
	}, nil
}

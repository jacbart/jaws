package envmanager

import (
	"errors"
	"fmt"
	"io"
	"log"
	"path/filepath"
	"strings"

	"github.com/hashicorp/hcl/v2"
	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/utils/style"
	"github.com/zclconf/go-cty/cty"
)

const (
	LABEL_PADDING       = 10
	NO_COMMENT          = ""
	JSON_COMMENT        = NO_COMMENT
	YAML_COMMENT        = "#"
	ENV_COMMENT         = "#"
	TF_COMMENT          = "#"
	INTERPRETER_SHEBANG = "#!jaws"
)

// ProcessConfigs - create an environment file
func (e *EnvHCL) Process(secrets []secretsmanager.Secret) error {
	var usedKeys []string
	var commentSymbol string
	var content string
	var evalContext *hcl.EvalContext
	var err error
	var format string
	envVarCount := 0
	if e.Message == "" {
		e.Message = "managed by jaws DO NOT EDIT"
	}

	var ext string
	if e.OutFormat != "" {
		ext = "." + e.OutFormat
	} else {
		if e.OutFormat != "" {
			ext = "." + e.OutFormat
		} else {
			ext = filepath.Ext(e.OutFile)
		}
	}
	switch ext {
	case ".yaml", ".yml":
		format = "yaml"
		commentSymbol = YAML_COMMENT
	case ".json":
		format = "json"
		commentSymbol = JSON_COMMENT
	case ".tfvars":
		format = "tfvars"
		commentSymbol = TF_COMMENT
	default:
		format = ""
		commentSymbol = ENV_COMMENT
	}

	if commentSymbol != "" {
		content = fmt.Sprintf("# %s\n", e.Message)
	}

	// parse the config file and return a *hcl.File
	srcHCL, diag := parseConfigFile(e)
	if diag.HasErrors() {
		return diag
	}

	if secrets != nil && e.Filter != "" {
		lastChar := e.Filter[len(e.Filter)-1:]
		if lastChar == "*" {
			e.Filter = strings.TrimSuffix(e.Filter, "*")
		} else if lastChar != "/" {
			e.Filter = fmt.Sprintf("%s/", e.Filter)
		}
		evalContext, err = createEnvHCLContext(e, srcHCL, secrets, []string{e.Filter})
		if err != nil {
			return fmt.Errorf(
				"error creating HCL evaluation context for envmanager: %w", err,
			)
		}
	} else {
		evalContext, err = createEnvHCLContext(e, srcHCL, nil, []string{""})
		if err != nil {
			return fmt.Errorf(
				"error creating HCL evaluation context for envmanager: %w", err,
			)
		}
	}

	// Insert top comments to output
	if commentSymbol != "" {
		if e.Profile != "" {
			content = fmt.Sprintf("%s%s profile: %s\n", content, commentSymbol, e.Profile)
		}
		if e.Filter != "" {
			content = fmt.Sprintf("%s%s prefix: %s\n", content, commentSymbol, e.Filter)
		}
		if e.Profile != "" || e.Filter != "" {
			content = fmt.Sprintf("%s\n", content)
		}
	}

	switch format {
	case "json":
		content = fmt.Sprintf("%s{\n", content)
	case "tfvars":
		content = fmt.Sprintf("%s[", content)
	default:
	}

	gvlen := len(e.GroupedVars)
	glvlen := len(e.GroupedLabeledVars)
	secretsDetected := false

	if glvlen > 0 {
		for _, group := range e.GroupedLabeledVars {
			if group.Label != "" {
				content = wrapGroupLabel(content, group.Label, commentSymbol)
			}
			err := processAttr(group.TmplVars, &usedKeys, &content, evalContext, &envVarCount, format)
			if err != nil {
				return err
			}
		}
		secretsDetected = true
	}
	if gvlen > 0 {
		for _, group := range e.GroupedVars {
			err := processAttr(group.TmplVars, &usedKeys, &content, evalContext, &envVarCount, format)
			if err != nil {
				return err
			}
		}
		secretsDetected = true
	}

	if !secretsDetected {
		for _, s := range secrets {
			s.ID = formatEnvVar(s.ID, e.Filter, "_", []string{"/", "-"})
			value := strings.TrimRight(s.Content, "\n")
			content = writeKeyValue(content, format, s.ID, value, &envVarCount)
			envVarCount++
		}
	}

	switch format {
	case "json":
		content = fmt.Sprintf("%s\n}\n", content)
	case "tfvars":
		content = fmt.Sprintf("%s\n]\n", content)
	default:
	}

	if envVarCount == 0 {
		content = ""
	}
	r := strings.NewReader(content)
	e.Reader = io.Reader(r)
	e.Processed = true
	return nil
}

func processAttr(vars hcl.Attributes, usedKeys *[]string, content *string, evalContext *hcl.EvalContext, envVarCount *int, format string) error {
	for _, envVar := range vars {
		v, diag := envVar.Expr.Value(evalContext)
		if diag.HasErrors() {
			if diag.HasErrors() {
				if strings.Contains(diag.Error(), "Unsupported attribute") {
					return errors.New(style.FailureString("no value for key '" + envVar.Name + "' found"))
				}
				return fmt.Errorf(
					"error in e.Write getting value of attribute HCL: %w", diag,
				)
			}
		}

		if !contains(*usedKeys, envVar.Name) {
			log.Default().Println("envmanager: processing attr", envVar.Name)
			log.Default().Println("envmanager: attr type", v.Type())
			switch v.Type() {
			case cty.Bool:
				log.Default().Println("envmanager: bool type")
			case cty.Number:
				log.Default().Println("envmanager: type int:", v.AsBigFloat())
				// *content = writeKeyValue(*content, format, envVar.Name, "", envVarCount)
			case cty.String:
				log.Default().Println("envmanager: string type")
				if strings.Contains(v.AsString(), FILE_FUNC_SUCCESS) {
					pathName := fmt.Sprintf("%s_PATH", envVar.Name)
					vStr := v.AsString()
					vStr = strings.ReplaceAll(vStr, FILE_FUNC_SUCCESS, "")
					log.Default().Println("envmanager: pathValue =", vStr)
					*content = writeKeyValue(*content, format, pathName, vStr, envVarCount)
				} else if strings.Contains(v.AsString(), INTERPRETER_SHEBANG) {
					log.Default().Println("envmanager: jaws script detected in", envVar.Name)
					sSplit := strings.SplitAfter(v.AsString(), "\n")
					alteredSecret := strings.Join(sSplit[1:], "")
					// eval secret
					updatedSecretContent, err := parseAttrString(alteredSecret)
					if err != nil {
						log.Default().Fatal(err)
					}
					*content = writeKeyValue(*content, format, envVar.Name, updatedSecretContent, envVarCount)
				} else {
					value := strings.TrimRight(v.AsString(), "\n")
					value = fmt.Sprintf("\"%s\"", value)
					*content = writeKeyValue(*content, format, envVar.Name, value, envVarCount)
				}
			default:
				log.Default().Println("envmanager: unknown type")
			}
			*usedKeys = append(*usedKeys, envVar.Name)
			*envVarCount++
		}
	}
	switch format {
	case "json", "tfvars":
	default:
		*content = fmt.Sprintf("%s\n", *content)
	}
	return nil
}

func writeKeyValue(content, format, key, value string, envVarCount *int) string {
	switch format {
	case "yaml":
		content = fmt.Sprintf("%s%s: %s\n", content, key, value)
	case "json":
		if *envVarCount == 0 {
			content = fmt.Sprintf("%s\n\t\"%s\": %s", content, key, value)
		} else {
			content = fmt.Sprintf("%s,\n\t\"%s\": %s", content, key, value)
		}
	case "tfvars":
		if *envVarCount == 0 {
			content = fmt.Sprintf("%s\n\t{\n\t\tname = \"%s\"\n", content, key)
		} else {
			content = fmt.Sprintf("%s,\n\t{\n\t\tname = \"%s\"\n", content, key)
		}
		content = fmt.Sprintf("%s\t\tvalue = %s\n\t}", content, value)
	default:
		content = fmt.Sprintf("%s%s=%s\n", content, key, value)
	}
	return content
}

// wrapGroupLabel formats the label using the commentSymbol as a filler
func wrapGroupLabel(content, title, commentSymbol string) string {
	if commentSymbol != "" {
		len := len(title) + LABEL_PADDING
		commentBuffer := strings.Repeat(commentSymbol, len)
		content = fmt.Sprintf("%s%s\n%s   %s   %s\n%s\n", content, commentBuffer, commentSymbol+commentSymbol, title, commentSymbol+commentSymbol, commentBuffer)
	}
	return content
}

func formatEnvVar(envVar, prefix, replacer string, separators []string) string {
	envVar = strings.TrimPrefix(envVar, prefix)
	envVar = strings.ToUpper(envVar)
	for _, sep := range separators {
		envVar = strings.ReplaceAll(envVar, sep, replacer)
	}
	return envVar
}

func contains(s []string, e string) bool {
	for _, a := range s {
		if a == e {
			return true
		}
	}
	return false
}

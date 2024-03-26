package envmanager

import (
	"log"
	"os"
	"strings"

	"github.com/hashicorp/hcl/v2"
	"github.com/hashicorp/hcl/v2/gohcl"
	"github.com/hashicorp/hcl/v2/hclparse"
	"github.com/hashicorp/hcl/v2/hclsyntax"
	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/zclconf/go-cty/cty"
)

// decodeSecretsVars - returns a map the secret variables
func decodeSecretVars(e *EnvHCL, srcHCL *hcl.File, localVars map[string]cty.Value, secrets []secretsmanager.Secret, prefixes []string) (map[string]cty.Value, hcl.Diagnostics) {
	var diags hcl.Diagnostics

	// create hcl context
	evalSecretContext, err := createSecretContext(localVars)
	if err != nil {
		diags = append(diags, &hcl.Diagnostic{
			Severity: hcl.DiagError,
			Summary:  "create local context",
			Detail:   err.Error(),
			Subject:  nil,
		})
		return cty.NilVal.AsValueMap(), diags
	}

	// decode env.ConfigFile using evalSecretContext
	envHCL := &EnvHCL{}
	if diag := gohcl.DecodeBody(srcHCL.Body, evalSecretContext, envHCL); diag.HasErrors() {
		deErr := &DecodeEnvFailed{File: e.ConfigFile}
		diags = append(diags, &hcl.Diagnostic{
			Severity: hcl.DiagError,
			Summary:  diag.Error(),
			Detail:   deErr.Error(),
			Subject:  nil,
		})
		return cty.NilVal.AsValueMap(), diags
	}

	secretsMap := make(map[string]cty.Value)
	if len(secrets) != 0 {
		for _, s := range secrets {
			for _, strPrefix := range prefixes {
				s.ID = strings.TrimPrefix(s.ID, strPrefix)
			}
			s.ID = formatEnvVar(s.ID, "/", "_", []string{"-", "/"})
			secretsMap[s.ID] = cty.StringVal(s.Content)
		}
	}

	return secretsMap, nil
}

// decodeEnvVars - returns a map of environment variables
func decodeEnvVars() map[string]cty.Value {
	// Extract all environment variables prefixed with JAWS_
	prefixedEnvVars := map[string]cty.Value{}
	for _, e := range os.Environ() {
		e := strings.SplitN(e, "=", 2)
		if len(e) != 2 {
			continue
		}
		key := e[0]
		value := e[1]

		if strings.HasPrefix(key, ENV_PREFIX) {
			key := strings.TrimPrefix(key, ENV_PREFIX)
			prefixedEnvVars[key] = cty.StringVal(value)
		}
	}
	return prefixedEnvVars
}

func decodeLocalVars(e *EnvHCL, srcHCL *hcl.File, prefixedEnvVars map[string]cty.Value) (map[string]cty.Value, hcl.Diagnostics) {
	var diags hcl.Diagnostics

	// create hcl context
	evalLocalContext, err := createLocalContext(prefixedEnvVars)
	if err != nil {
		diags = append(diags, &hcl.Diagnostic{
			Severity: hcl.DiagError,
			Summary:  "create local context",
			Detail:   err.Error(),
			Subject:  nil,
		})
		return cty.NilVal.AsValueMap(), diags
	}

	// decode env.ConfigFile using evalLocalContext
	envHCL := &EnvHCL{}
	if diag := gohcl.DecodeBody(srcHCL.Body, evalLocalContext, envHCL); diag.HasErrors() {
		deErr := &DecodeEnvFailed{File: e.ConfigFile}
		diags = append(diags, &hcl.Diagnostic{
			Severity: hcl.DiagError,
			Summary:  diag.Error(),
			Detail:   deErr.Error(),
			Subject:  nil,
		})
		return cty.NilVal.AsValueMap(), diags
	}
	e.Locals = append(e.Locals, envHCL.Locals...)

	locals := make(map[string]cty.Value)
	for _, l := range e.Locals {
		tmp, diag := decodeAttrs(l.TmplVars, evalLocalContext)
		if diag.HasErrors() {
			diags = append(diags, diag...)
		}

		for k, v := range tmp {
			locals[k] = v
		}
	}
	return locals, nil
}

func decodeAttrs(attrs hcl.Attributes, evalContext *hcl.EvalContext) (map[string]cty.Value, hcl.Diagnostics) {
	var diags hcl.Diagnostics

	keyValues := map[string]cty.Value{}
	for name, attr := range attrs {
		var val cty.Value
		val, diags = attr.Expr.Value(evalContext)
		if !hclsyntax.ValidIdentifier(name) {
			diags = append(diags, &hcl.Diagnostic{
				Severity: hcl.DiagError,
				Summary:  "Invalid local value name",
				Detail:   "asdf",
				Subject:  &attr.NameRange,
			})
		}
		keyValues[name] = val
	}

	return keyValues, diags
}

func parseConfigFile(e *EnvHCL) (*hcl.File, hcl.Diagnostics) {
	var diags hcl.Diagnostics
	// check if env config exists
	err := checkForEnvFile(e.ConfigFile)
	if err != nil {
		diags = append(diags, &hcl.Diagnostic{
			Severity: hcl.DiagError,
			Summary:  "no config file found",
			Detail:   err.Error(),
			Subject:  nil,
		})
		return nil, diags
	}
	// open and read the config file into a []byte
	envConfig, err := os.ReadFile(e.ConfigFile)
	if err != nil {
		diags = append(diags, &hcl.Diagnostic{
			Severity: hcl.DiagError,
			Summary:  "Unable to os.ReadFile",
			Detail:   err.Error(),
			Subject:  nil,
		})
		return nil, diags
	}

	// open and parse env.ConfigFile
	parser := hclparse.NewParser()
	srcHCL, diag := parser.ParseHCL(envConfig, e.ConfigFile)
	if diag.HasErrors() {
		return nil, diag
	}
	return srcHCL, nil
}

// parseAttrString re-evaluates a string using hcl
func parseAttrString(attr string) (string, error) {
	tmpFile, err := os.CreateTemp("", "")
	if err != nil {
		return "", err
	}
	fileName := tmpFile.Name()
	defer func() {
		tmpFile.Close()
		os.Remove(fileName)
	}()

	if strings.Contains(attr, "(") && strings.Contains(attr, ")") {
		attr = "vars {\nkey = " + attr + "\n}\n"
	} else {
		attr = "vars {\nkey = \"" + attr + "\"\n}\n"
	}
	log.Default().Println(attr)
	b := []byte(attr)
	if _, err = tmpFile.Write(b); err != nil {
		return "", err
	}

	parser := hclparse.NewParser()
	srcHCL, diag := parser.ParseHCL(b, fileName)
	if diag.HasErrors() {
		return "", diag
	}

	envVars := decodeEnvVars()

	evalContext, err := createLocalContext(envVars)
	if err != nil {
		return "", err
	}

	tmpHCL := &EnvHCL{}
	if diag := gohcl.DecodeBody(srcHCL.Body, evalContext, tmpHCL); diag.HasErrors() {
		return "", diag
	}

	var updatedAttrStr string
	for _, group := range tmpHCL.GroupedVars {
		for _, gVar := range group.TmplVars {
			v, diag := gVar.Expr.Value(evalContext)
			if diag.HasErrors() {
				return "", diag
			}
			updatedAttrStr = v.AsString()
			log.Default().Println("evaluated secret content:", updatedAttrStr)
		}
	}

	return updatedAttrStr, nil
}

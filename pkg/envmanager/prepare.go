package envmanager

import (
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/hashicorp/hcl/v2"
	"github.com/hashicorp/hcl/v2/gohcl"
	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/utils"
)

// Prepare env config and return a string slice with all required secrets and an error
func (e *EnvConfig) Prepare() error {
	includeCount := 0
	var secretIds []string
	for _, env := range e.Env {
		if env.Prepared {
			continue
		}
		log.Default().Println("envmanager: preparing", env.ConfigFile)

		// parse the config file and return a *hcl.File
		srcHCL, diag := parseConfigFile(env)
		if diag.HasErrors() {
			return diag
		}

		// create hcl context
		evalEnvHCLContext, err := createEnvHCLContext(env, srcHCL, []secretsmanager.Secret{}, []string{})
		if err != nil {
			return fmt.Errorf(
				"error creating HCL evaluation context for envmanager: %w", err,
			)
		}

		// decode env.ConfigFile using evalEnvHCLContext
		envHCL := &EnvHCL{}
		if diag := gohcl.DecodeBody(srcHCL.Body, evalEnvHCLContext, envHCL); diag.HasErrors() {
			deErr := &DecodeEnvFailed{File: env.ConfigFile}
			return fmt.Errorf("%s %w", deErr.Error(), diag)
		}

		// extract variables from envHCL and assign to e.Env's
		f := filepath.Base(env.ConfigFile)
		dir := strings.TrimSuffix(env.ConfigFile, f)
		var o string
		if strings.HasPrefix(envHCL.OutFile, "./") {
			o = dir + strings.TrimPrefix(envHCL.OutFile, "./")
		} else if strings.HasPrefix(envHCL.OutFile, "..") {
			o = dir + strings.TrimPrefix(envHCL.OutFile, "./")
		} else if strings.HasPrefix(envHCL.OutFile, "/") {
			o = envHCL.OutFile
		} else {
			o = dir + envHCL.OutFile
		}
		env.OutFile = o
		env.OutFormat = envHCL.OutFormat
		env.Message = envHCL.Message
		env.Profile = envHCL.Profile
		env.GroupedVars = append(env.GroupedVars, envHCL.GroupedVars...)
		env.GroupedLabeledVars = append(env.GroupedLabeledVars, envHCL.GroupedLabeledVars...)
		env.Locals = append(env.Locals, envHCL.Locals...)
		if envHCL.Filter != "" {
			// filter, err := parseAttrString(envHCL.Filter)
			// if err != nil {
			// 	return err
			// }
			// env.Filter = utils.FormatPrefixString(filter)
			if e.Options.FilterOverride != "" {
				env.Filter = utils.FormatPrefixString(e.Options.FilterOverride)
			} else {
				env.Filter = utils.FormatPrefixString(envHCL.Filter)
			}
		}
		log.Default().Println("envmanager: filter set to", env.Filter)
		env.Prepared = true
		for _, gv := range envHCL.GroupedVars {
			for _, v := range gv.TmplVars {
				for _, t := range v.Expr.Variables() {
					if t.RootName() == SECRET_KEY {
						split := t.SimpleSplit()
						for _, tr := range split.Rel {
							switch trType := tr.(type) {
							case hcl.TraverseAttr:
								name := (tr.(hcl.TraverseAttr)).Name
								sID := strings.TrimSuffix(env.Filter, "*") + strings.ToLower(name)
								sID = strings.ReplaceAll(sID, "_", "-")
								secretIds = append(secretIds, sID)
							default:
								return fmt.Errorf("unknown type: %v", trType)
							}
						}
					}
				}
			}
		}
		for _, gv := range envHCL.GroupedLabeledVars {
			for _, v := range gv.TmplVars {
				for _, t := range v.Expr.Variables() {
					if t.RootName() == SECRET_KEY {
						split := t.SimpleSplit()
						for _, tr := range split.Rel {
							switch trType := tr.(type) {
							case hcl.TraverseAttr:
								name := (tr.(hcl.TraverseAttr)).Name
								sID := strings.TrimSuffix(env.Filter, "*") + strings.ToLower(name)
								sID = strings.ReplaceAll(sID, "_", "-")
								secretIds = append(secretIds, sID)
							default:
								return fmt.Errorf("unknown type: %v", trType)
							}

						}
					}
				}
			}
		}

		// check for includes in the env config
		if len(envHCL.Includes) > 0 {
			log.Default().Println("envmanager: includes set to", envHCL.Includes)
			for _, include := range envHCL.Includes {
				if !strings.HasPrefix(include, "/") {
					dir, err := filepath.Abs(filepath.Dir(env.ConfigFile))
					if err != nil {
						return err
					}
					include = dir + "/" + strings.TrimPrefix(include, "./")
				}
				// check if include is a directory
				info, err := os.Stat(include)
				if err != nil {
					if errors.Is(err, os.ErrNotExist) {
						return fmt.Errorf("%s was not found", include)
					} else {
						return err
					}
				}
				if info.IsDir() {
					err = e.SearchDir(include)
					if err != nil {
						return err
					}
					includeCount++
				} else {
					err = e.AddEnvConfig(include)
					if err != nil {
						return err
					}
					includeCount++
				}
			}
		}
	}
	if includeCount > 0 {
		err := e.Prepare()
		if err != nil {
			return err
		}
	}
	for _, id := range secretIds {
		if !contains(e.SecretIDs, id) {
			e.SecretIDs = append(e.SecretIDs, id)
		}
	}
	return nil
}

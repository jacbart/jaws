package main

import (
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/jacbart/jaws/pkg/envmanager"
	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/tui"
	"github.com/spf13/cobra"
)

const (
	defaultOutfile = ".env"
	defaultFormat  = ""
)

func PullCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "pull",
		Short: "pull latest secrets, if no secret is specified use a fuzzyfinder to select secret(s)",
		Long: `pull latest secrets, if no secret is specified jaws loads the list of secrets into
a fuzzyfinder, you can then search for secrets by typing, select secrets with tab and enter to confirm
selected secrets to download them. When specifying a secret from the cli you can end it in / or /* to
grab all secrets with that prefix`,
		Example: "jaws pull testing/app/default/key --print",
		Aliases: []string{"get"},
		RunE: func(cmd *cobra.Command, args []string) error {
			noSelErr := errors.New("no secrets selected")
			var secretIds []string
			var err error
			var noOutFileErr *NoOutputFileSet
			opts := envmanager.Options{
				Diff:           diffEnv,
				Overwrite:      overwriteEnv,
				UnsafeMode:     disabledSafeEnv,
				FilterOverride: envFilter,
			}
			if disableDetectJawsFiles {
				jawsConf.Conf.General.DisableDetectJawsFiles = true
			}

			if inputEnvFile != "" || outputEnvFile != defaultOutfile { // if the --in or --out flag has been changed
				// if the user set the output file to "", error out
				if outputEnvFile == "" {
					return noOutFileErr
				}

				// initalize an envmanager.EnvConfig struct
				env := envmanager.InitEnv(&opts)

				// add input file
				if inputEnvFile != "" {
					err = env.AddEnvConfig(inputEnvFile)
					if err != nil {
						return err
					}
				} else {
					err = env.SearchDir(".")
					if err != nil {
						return err
					}
				}

				// Prep env and grab all needed secrets
				err = env.Prepare()
				if err != nil {
					return err
				}

				var secrets []secretsmanager.Secret
				// pull all needed secrets
				if len(env.SecretIDs) > 0 {
					err := secretManager.SecretSelect(env.SecretIDs)
					if err != nil {
						return err
					}
					for _, e := range env.Env {
						newSecrets, err := secretManager.Pull(e.Filter)
						if err != nil {
							return err
						}
						secrets = append(secrets, newSecrets...)
					}
				}

				for _, e := range env.Env {
					if e.OutFile == "" || outputEnvFile != defaultOutfile {
						e.OutFile = outputEnvFile
					}
					if outFormat != defaultFormat {
						e.OutFormat = outFormat
					}
					// Process each env file
					err = e.Process(secrets)
					if err != nil {
						return err
					}

				}
				// write the env file or print to stdout if output is set to -
				err = env.Write()
				if err != nil {
					return err
				}

			} else {
				var jawsFile string
				var present bool
				if jawsConf.Conf.General.DisableDetectJawsFiles {
					present = false
				} else {
					jawsFile, present = checkDotJawsFile()
				}
				if present { // if a .jaws file is present in the current directory
					// if the user set the output file to "", error out
					if outputEnvFile == "" {
						return noOutFileErr
					}

					// handle if jaws file is detected and selector is run but the user quits
					if jawsFile == "" {
						return nil
					}

					// initalize an envmanager.EnvConfig struct
					env := envmanager.InitEnv(&opts)

					// add input file
					err = env.AddEnvConfig(jawsFile)
					if err != nil {
						return err
					}

					// Prep env and grab all needed secrets
					err = env.Prepare()
					if err != nil {
						return err
					}

					var secrets []secretsmanager.Secret
					// pull all needed secrets
					if len(env.SecretIDs) > 0 {
						err := secretManager.SecretSelect(env.SecretIDs)
						if err != nil {
							return err
						}
						for _, e := range env.Env {
							newSecrets, err := secretManager.Pull(e.Filter)
							if err != nil {
								return err
							}
							secrets = append(secrets, newSecrets...)
						}
					}

					for _, e := range env.Env {
						if e.OutFile == "" || outputEnvFile != defaultOutfile {
							e.OutFile = outputEnvFile
						}
						if outFormat != defaultFormat {
							e.OutFormat = outFormat
						}
						// Process each env file
						err = e.Process(secrets)
						if err != nil {
							return err
						}
					}

					// write the env file or print to stdout if output is set to -
					err = env.Write()
					if err != nil {
						return err
					}
				} else {
					err := secretManager.SecretSelect(args)
					if err != nil {
						return err
					}
					var secrets []secretsmanager.Secret
					if len(args) > 0 {
						for _, arg := range args {
							prefix := ""
							if utils.CheckIfPrefix(arg) {
								prefix = arg
							}

							newSecrets, err := secretManager.Pull(prefix)
							if err != nil {
								return err
							}
							secrets = append(secrets, newSecrets...)
						}
					} else {
						newSecrets, err := secretManager.Pull("")
						if err != nil {
							return nil
						}
						secrets = append(secrets, newSecrets...)
					}

					if print { // if the print flag is set
						secretsmanager.PrintSecrets(secrets)
					} else { // if no print flag was set, download the secrets
						for _, s := range secrets {
							log.Default().Println("Downloading:", s.ID)
							err = utils.DownloadSecret(
								s.ID,
								s.Content,
								fmt.Sprintf("%s/%s", secretsPath, secretManager.Platform()),
								"/",
							)
							if err != nil {
								return err
							}
							secretIds = append(secretIds, s.ID)
							fmt.Printf("%s/%s/%s\n", secretsPath, secretManager.Platform(), s.ID)
						}
						f, err := filepath.Abs(fmt.Sprintf("%s/%s", secretsPath, secretManager.Platform()))
						if err != nil {
							return err
						}
						baseOfPath := fmt.Sprintf("/%s", filepath.Base(f))
						parentPath := strings.TrimSuffix(f, baseOfPath)
						_ = utils.CheckIfGitRepo(parentPath, jawsConf.Conf.General.RepoWarn)
						utils.GitControlSecrets(secretIds, fmt.Sprintf("%s/%s", secretsPath, secretManager.Platform()))
						if useEditor {
							if err = utils.OpenWithEditor(
								secretIds,
								fmt.Sprintf("%s/%s", secretsPath, secretManager.Platform()),
							); err != nil {
								if err.Error() != noSelErr.Error() {
									return err
								}
							}
						}
					}
				}
			}
			return nil
		},
	}
}

// checkDotJawsFile returns the file name of a file ending in .jaws if it exists and a bool
func checkDotJawsFile() (string, bool) {
	detected := false
	// list all files in current directory
	files, err := os.ReadDir(".")
	if err != nil {
		log.Default().Fatal(err)
	}
	jawsFile := ""
	var jawsFiles []string
	// search for file ending in .jaws
	for _, file := range files {
		if strings.Contains(file.Name(), ".jaws") {
			jawsFiles = append(jawsFiles, file.Name())
		}
	}

	if len(jawsFiles) > 1 {
		jawsFile, err = tui.SelectorTUI(jawsFiles)
		if err != nil {
			return "", detected
		}
		detected = true
	} else if len(jawsFiles) > 0 {
		jawsFile = jawsFiles[0]
	}

	if jawsFile != "" {
		info, _ := os.Stat(jawsFile)
		if info.IsDir() {
			return "", detected
		}
		detected = true
		return jawsFile, detected
	}
	return "", detected
}

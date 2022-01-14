package main

import (
	"errors"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/utils/helpers"
	"github.com/spf13/cobra"
)

func main() {
	cobra.CheckErr(rootCmd.Execute())
}

func commands() {
	// add path command and sub commands
	rootCmd.AddCommand(pathCmd)
	pathCmd.AddCommand(pathCommandCmd)
	// add clean command
	rootCmd.AddCommand(cleanCmd)
	// add create command
	rootCmd.AddCommand(createCmd)
	// add delete command and sub cancel command
	rootCmd.AddCommand(deleteCmd)
	deleteCmd.AddCommand(deleteCancelCmd)
	// add diff command
	rootCmd.AddCommand(diffCmd)
	// add status command
	rootCmd.AddCommand(statusCmd)
	// add get command
	rootCmd.AddCommand(getCmd)
	// add list command
	rootCmd.AddCommand(listCmd)
	// add rollback command
	rootCmd.AddCommand(rollbackCmd)
	// add set command
	rootCmd.AddCommand(setCmd)
}

func flags() {
	// global persistent flags
	rootCmd.PersistentFlags().StringVar(&secretsPath, "path", "secrets", "sets download path for secrets, overrides config")
	rootCmd.PersistentFlags().StringVarP(&cfgFile, "config", "c", "", "set config file")
	// create command flags
	createCmd.Flags().BoolVarP(&useEditor, "editor", "e", false, "open any selected secrets in an editor")
	// delete command flags
	deleteCmd.Flags().Int64Var(&scheduleInDays, "days", 30, "set time till deletion in days, minimum 7")
	// get command flags
	getCmd.Flags().BoolVarP(&cleanPrintValue, "print", "p", false, "print secret string to terminal instead of downloading to a file")
	getCmd.Flags().BoolVarP(&formatPrintValue, "fmt-print", "f", false, "print formatted secret string to terminal instead of downloading to a file")
	getCmd.Flags().BoolVarP(&useEditor, "editor", "e", false, "open any selected secrets in an editor")
	// set command flags
	setCmd.Flags().BoolVar(&createPrompt, "no-prompt", false, "add this flag to skip the confirmation prompt of new secrets")
	setCmd.Flags().BoolVarP(&cleanLocalSecrets, "keep-secrets", "k", false, "set to keep secrets after pushing/setting them")
}

var (
	secretManager     secretsmanager.Manager
	cfgFile           string
	secretsPath       string
	scheduleInDays    int64
	useEditor         bool
	formatPrintValue  bool
	cleanPrintValue   bool
	createPrompt      bool
	cleanLocalSecrets bool

	// rootCmd represents the base command when called without any subcommands
	rootCmd = &cobra.Command{
		Use:   "firm",
		Short: "firm (Fidelius Charm) is a cli tool to interact with secrets managers",
		Long: `firm (Fidelius Charm) is a cli tool to interact with secrets managers.
A recommened secrets format is ENV/APP/DEPLOYMENT/SecretType. When downloading
secrets they will create a path using the name of the secret, it requires the same format when uploading secrets.`,
		Example: "firm get --print",
	}

	// pathCmd represents the set command
	pathCmd = &cobra.Command{
		Use:     "path",
		Short:   "prints path to secrets folder and will create the path if it does not exist",
		Example: "cd $(firm path)",
		RunE: func(cmd *cobra.Command, args []string) error {
			return helpers.Path(secretsPath)
		},
	}

	// pathCommandCmd represents the path command command
	pathCommandCmd = &cobra.Command{
		Use:     "command",
		Short:   "prints out the shell function that lets firm-cd work properly",
		Example: "source <(firm path command)",
		Run: func(cmd *cobra.Command, args []string) {
			helpers.PathCommand()
		},
	}

	// cleanCmd represents the set command
	cleanCmd = &cobra.Command{
		Use:     "clean",
		Short:   "clean the local secrets from your computer, same as 'rm -rf /path/to/secrets'",
		Aliases: []string{"scrub"},
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretsmanager.Clean(secretsPath)
		},
	}

	// createCmd represents the set command
	createCmd = &cobra.Command{
		Use:   "create",
		Short: "creates folder path and empty file to edit",
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretManager.Create(args, secretsPath, useEditor)
		},
	}

	// deleteCmd represents the set command
	deleteCmd = &cobra.Command{
		Use:     "delete",
		Short:   "schedule secret(s) for deletion",
		Aliases: []string{"remove"},
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretManager.Delete(scheduleInDays)
		},
	}

	// deleteCancelCmd represents the delete sub command cancel
	deleteCancelCmd = &cobra.Command{
		Use:     "cancel",
		Short:   "cancel a scheduled secret deletion",
		Example: "firm delete cancel testing/app/default/secret",
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretManager.DeleteCancel(args)
		},
	}

	// diffCmd represents the set command
	diffCmd = &cobra.Command{
		Use:   "diff",
		Short: "uses git to compare original secret with the changed secret, you can run git diff in the secrets location to get the same results",
		RunE: func(cmd *cobra.Command, args []string) error {
			return helpers.GitDiff(secretsPath)
		},
	}

	// statusCmd represents the set command
	statusCmd = &cobra.Command{
		Use:   "status",
		Short: "uses git status to compare original secret with the changed secret",
		RunE: func(cmd *cobra.Command, args []string) error {
			return helpers.GitStatus(secretsPath)
		},
	}

	// getCmd represents the set command
	getCmd = &cobra.Command{
		Use:   "get",
		Short: "download or print secret from aws, if no secret is specified use fzf to select secret(s)",
		Long: `download or print secret from aws, if no secret is specified firm loads the list of secrets into
fzf, you can then search for secrets by typing, select secrets with tab and enter to confirm
selected secrets to download them.`,
		Example: "firm get testing/app/default/key -p",
		RunE: func(cmd *cobra.Command, args []string) error {
			var noSelErr = errors.New("no secrets selected")
			var secretIDs []string
			Secrets, err := secretManager.Get(args)
			if err != nil {
				return err
			}
			for _, s := range Secrets {
				secretIDs = append(secretIDs, s.ID)
			}

			if !formatPrintValue && !cleanPrintValue {
				f, err := filepath.Abs(secretsPath)
				if err != nil {
					return err
				}
				baseOfPath := fmt.Sprintf("/%s", filepath.Base(f))
				parentPath := strings.TrimSuffix(f, baseOfPath)
				_ = helpers.CheckIfGitRepo(parentPath, true)
				helpers.GitControlSecrets(secretIDs, secretsPath)
				if useEditor {
					if err = helpers.OpenEditor(secretIDs, secretsPath); err != nil {
						if err.Error() != noSelErr.Error() {
							return err
						}
					}
				}
			} else {
				if cleanPrintValue {
					secretsmanager.CleanPrintSecrets(Secrets)
				} else if formatPrintValue {
					secretsmanager.FormatPrintSecret(Secrets)
				}
			}
			return nil
		},
	}

	// listCmd represents the list command
	listCmd = &cobra.Command{
		Use:     "list",
		Short:   "list available secrets",
		Aliases: []string{"ls"},
		RunE: func(cmd *cobra.Command, args []string) error {
			list, err := secretManager.ListAll()
			for _, secretID := range list {
				fmt.Println(secretID)
			}
			return err
		},
	}

	// rollbackCmd represents the set command
	rollbackCmd = &cobra.Command{
		Use:     "rollback",
		Short:   "rollback the selected secrets by one version (only 2 total versions available)",
		Aliases: []string{"rotate"},
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretManager.Rollback()
		},
	}

	// setCmd represents the set command
	setCmd = &cobra.Command{
		Use:   "set",
		Short: "updates secrets and will prompt to create if there is a new secret detected",
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretManager.Set(secretsPath, createPrompt)
		},
		PostRunE: func(cmd *cobra.Command, args []string) error {
			return secretsmanager.SetPostRun(secretsPath, cleanLocalSecrets)
		},
	}
)

func init() {
	cobra.OnInitialize(initConfig)
	commands()
	flags()
}

// initConfig reads in config file and ENV variables if set.
func initConfig() {

	c := secretsmanager.NewFirmConfig()

	if cfgFile != "" {
		c.SetConfigName(cfgFile)
	} else {
		c.SetConfigName("firm.config")
		c.AddConfigPath(".")
		c.AddConfigPath(fmt.Sprintf("%s/.config/firm", os.Getenv("HOME")))
		c.AddConfigPath(os.Getenv("HOME"))
	}

	general, managers, err := c.ReadInConfig()
	if err != nil {
		switch err.(type) {
		case *secretsmanager.NoConfigFileFound:
			fmt.Println("no config found, defaulting to aws")
			secretManager = &secretsmanager.AWSManager{
				Profile: "default",
			}
			general = &secretsmanager.GeneralHCL{
				DefaultProfile: "default",
				SecretsPath:    "secrets",
			}
		default:
			log.Fatalln(err)
		}
	} else {
		if len(managers) != 0 {
			for _, m := range managers {
				if m.ProfileName() == general.DefaultProfile {
					secretManager = m
				}
			}
		}
	}

	// check if secretsPath flag is set to something other than secrets, if not then use config set path
	if strings.Compare(secretsPath, "secrets") == 0 {
		secretsPath = general.SecretsPath
	}
	if general.Editor != "" {
		os.Setenv("EDITOR", general.Editor)
	}
}

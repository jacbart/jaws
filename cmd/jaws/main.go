package main

import (
	"fmt"
	"io"
	"log"
	"os"
	"strings"

	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
)

func main() {
	cobra.CheckErr(rootCmd.Execute())
}

func init() {
	cobra.OnInitialize(InitConfig)
	Commands()
	Flags()
}

// CMD Variables
var (
	secretManager          secretsmanager.Manager
	jawsConf               secretsmanager.CliConfig
	cfgFile                string
	secretsPath            string
	profile                string
	useEditor              bool
	print                  bool
	inputEnvFile           string
	outputEnvFile          string
	createPrompt           bool
	cleanLocalSecrets      bool
	shortVersion           bool
	Version                string
	Date                   string
	diffEnv                bool
	overwriteEnv           bool
	disabledSafeEnv        bool
	checkUpdateOnly        bool
	recursiveSearch        bool
	outFormat              string
	disableDetectJawsFiles bool
	reencrypt              bool
	deleteCancel           bool
	debugMode              bool
	cicdMode               bool
	envFilter              string
)

// Cobra Commands
var (
	rootCmd         = RootCmd()
	addCmd          = AddCmd()
	cleanCmd        = CleanCmd()
	configCmd       = ConfigCmd()
	configPathCmd   = ConfigPathCmd()
	configShowCmd   = ConfigShowCmd()
	configCreateCmd = ConfigCreateCmd()
	configEditCmd   = ConfigEditCmd()
	deleteCmd       = DeleteCmd()
	diffCmd         = DiffCmd()
	listCmd         = ListCmd()
	configLockCmd   = ConfigLockCmd()
	configUnlockCmd = ConfigUnlockCmd()
	pathCmd         = PathCmd()
	pathCommandCmd  = PathCommandCmd()
	pullCmd         = PullCmd()
	pushCmd         = PushCmd()
	rollbackCmd     = RollbackCmd()
	statusCmd       = StatusCmd()
	updateCmd       = UpdateCmd()
	versionCmd      = VersionCmd()
)

// RootCmd
func RootCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "jaws",
		Short: "tool for interacting with secret managers and creating and managing variable files",
		Long: `tool for interacting with secret managers, plus creating and managing environemnt/variable files.
When downloading secrets jaws will create a path using the secret's name.`,
		SilenceUsage:  true,
		SilenceErrors: true,
		PersistentPreRun: func(cmd *cobra.Command, args []string) {
			if !debugMode {
				log.Default().SetOutput(io.Discard)
			}
		},
	}
}

// InitConfig reads in config file and ENV variables if set.
func InitConfig() {
	jawsConf = secretsmanager.InitCliConfig()

	if cfgFile != "" {
		jawsConf.SetConfigName(cfgFile)
	} else {
		jawsConf.SetConfigName("jaws.conf")
		jawsConf.AddConfigPath(".")
		jawsConf.AddConfigPath(fmt.Sprintf("%s/.jaws", os.Getenv("HOME")))
		jawsConf.AddConfigPath(fmt.Sprintf("%s/.config/jaws", os.Getenv("HOME")))
	}

	managers, err := jawsConf.ReadInConfig()
	if err != nil {
		switch err.(type) {
		case *secretsmanager.NoConfigFileFound:
			// 	log.Default().Println("no config found, defaulting to aws")
			secretManager = &secretsmanager.AWSManager{
				Profile: "default",
			}
			jawsConf.Conf.General = secretsmanager.GeneralHCL{
				DefaultProfile: "default",
			}
		case *secretsmanager.DecodeConfigFailed:
			secretManager = &secretsmanager.AWSManager{
				Profile: "default",
			}
			jawsConf.Conf.General = secretsmanager.GeneralHCL{
				DefaultProfile: "default",
			}
		default:
			log.Default().Fatalln(err)
		}
	} else {
		// log.Default().Println("config loaded from", jawsConf.CurrentConfig)
		if len(managers) > 0 {
			var profiles []string
			if profile != "" { // if profile flag is set then override the default profile
				jawsConf.Conf.General.DefaultProfile = profile
			}
			for _, m := range managers {
				profiles = append(profiles, m.ProfileName())
				if m.ProfileName() == jawsConf.Conf.General.DefaultProfile {
					secretManager = m
					// 			log.Default().Println("config load: profile set to", jawsConf.Conf.General.DefaultProfile)
				}
			}
			if secretManager == nil { // no profile found
				fmt.Printf("profile '%s' not found in %s\n", style.FailureString(jawsConf.Conf.General.DefaultProfile), jawsConf.CurrentConfig)
				fmt.Printf("  Available Profiles:\n")
				for _, p := range profiles {
					fmt.Printf("  %s\n", style.InfoString(p))
				}
				os.Exit(1)
			}
		}
	}

	// Flag default overrides from config file

	// check if secretsPath flag is set to something other than secrets, if not then use config set path
	if strings.Compare(secretsPath, "secrets") == 0 {
		if jawsConf.Conf.General.SecretsPath != "" {
			secretsPath = jawsConf.Conf.General.SecretsPath
			// 	log.Default().Println("config load: setting secrets path to", jawsConf.Conf.General.SecretsPath)
		}
	}
	// if Editor is not set in conf file use the env var EDITOR
	if jawsConf.Conf.General.Editor != "" {
		os.Setenv("EDITOR", jawsConf.Conf.General.Editor)
		// log.Default().Println("config load: setting EDITIOR var to", jawsConf.Conf.General.Editor)
	}
	// if SafeMode in the config is false then set the disabledSafeEnv flag to true
	if !jawsConf.Conf.General.SafeMode {
		disabledSafeEnv = true
		// log.Default().Println("config load: disabling safe mode")
	}
	// if there is no gh_token set in the conf, default to the env var GH_TOKEN
	if jawsConf.Conf.General.GithubToken == "" {
		token, present := os.LookupEnv("GH_TOKEN")
		if present {
			jawsConf.Conf.General.GithubToken = token
			// 	log.Default().Println("config load: github token detected")
		}
	}
}

package main

import (
	"fmt"
	"io"
	"os"

	. "github.com/jacbart/jaws/cmd/jaws/config"
	"github.com/jacbart/jaws/pkg/lockandload"
	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
)

func ConfigCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "config",
		Short:   "display current config info",
		Aliases: []string{"conf"},
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Printf("Config:\t\t%s\n", jawsConf.CurrentConfig)
			fmt.Printf("Profile:\t%s\n", secretManager.ProfileName())
			fmt.Printf("Platform:\t%s\n", secretManager.Platform())
			fmt.Printf("Locale:\t\t%s\n", secretManager.Locale())
		},
	}
}

func ConfigPathCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "path",
		Short: "display current config path",
		Run: func(cmd *cobra.Command, args []string) {
			fmt.Println(jawsConf.CurrentConfig)
		},
	}
}

func ConfigShowCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "show",
		Short:   "Show current config",
		Aliases: []string{"display"},
		RunE: func(cmd *cobra.Command, args []string) error {
			f, err := lockandload.NewSecureFile(jawsConf.CurrentConfig, jawsConf.Key)
			if err != nil {
				return err
			}
			r, err := f.Load()
			if err != nil {
				return err
			}
			var w io.Writer = os.Stdout
			_, err = io.Copy(w, r)
			if err != nil {
				return err
			}
			return nil
		},
	}
}

func ConfigCreateCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "create",
		Short:   "Creates a new config file",
		Aliases: []string{"add", "gen", "generate"},
		RunE: func(cmd *cobra.Command, args []string) error {
			if cicdMode {
				return CreateConfig(nil)
			}
			c, err := SetupWizard()
			if err != nil {
				return err
			}

			return CreateConfig(&c)
		},
	}
}

func ConfigEditCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "edit",
		Short: "edit the current config file",
		PreRunE: func(cmd *cobra.Command, args []string) error {
			f, err := lockandload.NewSecureFile(jawsConf.CurrentConfig, jawsConf.Key)
			if err != nil {
				return err
			}
			if f.Locked {
				fmt.Printf("%s %s\n", style.InfoString("decrypting"), jawsConf.CurrentConfig)
				err = f.Decrypt()
				if err != nil {
					return err
				}
				reencrypt = true
			} else {
				reencrypt = false
			}
			return nil
		},
		RunE: func(cmd *cobra.Command, args []string) error {
			return utils.OpenWithEditor([]string{jawsConf.CurrentConfig}, "")
		},
		PostRunE: func(cmd *cobra.Command, args []string) error {
			if reencrypt {
				fmt.Printf("%s %s\n", style.InfoString("re-encrypting"), jawsConf.CurrentConfig)
				f, err := lockandload.NewSecureFile(jawsConf.CurrentConfig, jawsConf.Key)
				if err != nil {
					return err
				}
				err = f.Encrypt()
				if err != nil {
					return err
				}
			}
			return nil
		},
	}
}

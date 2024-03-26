package main

import (
	"fmt"

	"github.com/jacbart/jaws/pkg/lockandload"
	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
)

func ConfigLockCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "lock",
		Short: "encrypt the current config with a passphrase",
		RunE: func(cmd *cobra.Command, args []string) error {
			var l lockandload.SecureFile
			var err error

			lArgs := len(args)
			if lArgs <= 1 {
				if lArgs == 1 {
					l, err = lockandload.NewSecureFile(jawsConf.CurrentConfig, args[0])
					if err != nil {
						return err
					}
				} else {
					l, err = lockandload.NewSecureFile(jawsConf.CurrentConfig, jawsConf.Key)
					if err != nil {
						return err
					}
				}
			} else {
				return fmt.Errorf("lock only takes an optional passphrase argument")
			}
			err = l.Encrypt()
			if err != nil {
				fmt.Printf("%s encryption %s\n", jawsConf.CurrentConfig, style.FailureString("failed"))
				return err
			}
			fmt.Printf("%s encrypted %s\n", jawsConf.CurrentConfig, style.SuccessString("successfully"))
			return nil
		},
	}
}

func ConfigUnlockCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "unlock",
		Short: "Decrypt current config",
		RunE: func(cmd *cobra.Command, args []string) error {
			var l lockandload.SecureFile
			var err error
			lArgs := len(args)
			if lArgs <= 1 {
				if lArgs == 1 {
					l, err = lockandload.NewSecureFile(jawsConf.CurrentConfig, args[1])
					if err != nil {
						return err
					}
				} else {
					l, err = lockandload.NewSecureFile(jawsConf.CurrentConfig, jawsConf.Key)
					if err != nil {
						return err
					}
				}
			} else {
				return fmt.Errorf("unlock only takes an optional passphrase argument")
			}
			err = l.Decrypt()
			if err != nil {
				fmt.Printf("%s decryption %s\n", jawsConf.CurrentConfig, style.FailureString("failed"))
				return err
			}
			fmt.Printf("%s decrypted %s\n", jawsConf.CurrentConfig, style.SuccessString("successfully"))
			return nil
		},
	}
}

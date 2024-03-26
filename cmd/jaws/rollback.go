package main

import (
	"github.com/spf13/cobra"
)

func RollbackCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "rollback",
		Short: "rollback the selected secrets to a previous version",
		RunE: func(cmd *cobra.Command, args []string) error {
			err := secretManager.SecretSelect(args)
			if err != nil {
				return err
			}
			return secretManager.Rollback()
		},
	}
}

package main

import (
	"errors"

	"github.com/spf13/cobra"
)

func DeleteCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "delete",
		Short:   "delete secret(s) off the secrets manager",
		Aliases: []string{"del", "remove"},
		RunE: func(cmd *cobra.Command, args []string) error {
			secretManager.SecretSelect(args)

			var returnErr error
			switch secretManager.Platform() {
			case "aws":
				if deleteCancel {
					returnErr = secretManager.CancelDelete()
				} else {
					returnErr = secretManager.Delete()
				}
			case "gcp":
				returnErr = secretManager.Delete()
			default:
				returnErr = errors.New("unknown platform")
			}

			return returnErr
		},
	}
}

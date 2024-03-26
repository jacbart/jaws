package main

import (
	"github.com/jacbart/jaws/utils"
	"github.com/spf13/cobra"
)

func DiffCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "diff",
		Short: "uses git to compare original secret with the changed secret, you can run git diff in the secrets location to get the same results",
		RunE: func(cmd *cobra.Command, args []string) error {
			return utils.GitDiff(secretsPath)
		},
	}
}

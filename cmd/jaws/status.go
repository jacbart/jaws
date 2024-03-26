package main

import (
	"github.com/jacbart/jaws/utils"
	"github.com/spf13/cobra"
)

func StatusCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "status",
		Short: "uses git status to compare original secret with the changed secret",
		RunE: func(cmd *cobra.Command, args []string) error {
			return utils.GitStatus(secretsPath)
		},
	}
}

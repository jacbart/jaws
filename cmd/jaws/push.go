package main

import (
	"github.com/jacbart/jaws/utils"
	"github.com/spf13/cobra"
)

func PushCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "push",
		Short:   "pushes updated secrets and will prompt to create a secret if there is a new one detected",
		Aliases: []string{"set"},
		RunE: func(cmd *cobra.Command, args []string) error {
			return secretManager.Push(secretsPath+"/"+secretManager.Platform(), createPrompt)
		},
		PostRunE: func(cmd *cobra.Command, args []string) error {
			return utils.PushPostRun(secretsPath+"/"+secretManager.Platform(), cleanLocalSecrets)
		},
	}
}

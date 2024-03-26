package main

import (
	"fmt"
	"os"

	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
)

func CleanCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "clean",
		Short:   "deletes the secrets folder",
		Aliases: []string{"scrub"},
		RunE: func(cmd *cobra.Command, args []string) error {
			err := os.RemoveAll(secretsPath)
			if err != nil {
				return nil
			}
			fmt.Println(style.WarningString("folder"), style.WarningString(secretsPath), style.WarningString("deleted"))
			return nil
		},
	}
}

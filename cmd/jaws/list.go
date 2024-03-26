package main

import (
	"fmt"

	"github.com/spf13/cobra"
)

func ListCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "list",
		Short:   "list available secrets",
		Aliases: []string{"ls"},
		RunE: func(cmd *cobra.Command, args []string) error {
			var list []string
			if len(args) != 0 {
				for _, arg := range args {
					l := secretManager.ListAll(arg)
					list = append(list, l...)
				}
			} else {
				l := secretManager.ListAll("")
				list = append(list, l...)
			}

			for _, id := range list {
				fmt.Println(id)
			}
			return nil
		},
	}
}

package main

import (
	"context"
	"fmt"

	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
	"golang.org/x/oauth2"
)

func VersionCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "version",
		Short:   "display version and info on jaws binary",
		Aliases: []string{"v"},
		RunE: func(cmd *cobra.Command, args []string) error {
			if shortVersion {
				fmt.Println(Version)
			} else {
				fmt.Printf("jaws version %s (%s)\n", Version, Date)
				fmt.Println("https://github.com/jacbart/jaws/releases/tag/v" + Version)
			}
			if checkUpdateOnly {
				ctx, cancel := context.WithCancel(context.Background())
				defer cancel()

				// static token for github oauth2
				ts := oauth2.StaticTokenSource(
					&oauth2.Token{AccessToken: jawsConf.Conf.General.GithubToken},
				)
				// http client using oauth2
				tc := oauth2.NewClient(ctx, ts)

				nv, err := utils.GitCheckForUpdate(tc, ctx, Version)
				if err != nil {
					return err
				}
				if nv != nil {
					fmt.Printf("update available: %s\n", style.SuccessString(nv.String()))
				} else {
					fmt.Printf("%s: running latest or newer\n", style.InfoString("no updates"))
				}
			}
			return nil
		},
	}
}

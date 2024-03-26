package main

import (
	"context"
	"fmt"

	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
	"golang.org/x/oauth2"
)

func UpdateCmd() *cobra.Command {
	return &cobra.Command{
		Use:   "update",
		Short: "check for and update jaws to the latest release",
		RunE: func(cmd *cobra.Command, args []string) error {
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
				return nil
			}
			return utils.GitLatestRelease(Version, jawsConf.Conf.General.GithubToken)
		},
	}
}

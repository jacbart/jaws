package main

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"
	"strings"

	"github.com/jacbart/jaws/pkg/secretsmanager"
	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
	"github.com/spf13/cobra"
)

func AddCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "add",
		Short:   "creates folder path and empty file to edit",
		Aliases: []string{"create"},
		RunE: func(cmd *cobra.Command, args []string) error {
			var filePath string
			var dir string
			pattern := strings.Split(args[0], "/")
			switch secretManager.Platform() {
			case "aws":
				log.Default().Println("type is AWSManager")
				filePath = fmt.Sprintf("%s/%s", secretsPath+"/"+secretManager.Platform(), args[0])
				dir = fmt.Sprintf("%s/%s", secretsPath+"/"+secretManager.Platform(), strings.Join(pattern[:len(pattern)-1], "/"))
			case "gcp":
				log.Default().Println("type is GCPManager")
				g := secretManager.(*secretsmanager.GCPManager)
				_, err := secretsmanager.LoadGCPClient(g, context.Background())
				if err != nil {
					return err
				}
				args[0] = g.DefaultProject + "/secrets/" + args[0]
				filePath = fmt.Sprintf("%s/%s", secretsPath+"/"+secretManager.Platform(), args[0])
				dir = fmt.Sprintf("%s/%s", secretsPath+"/"+secretManager.Platform()+"/"+g.DefaultProject+"/secrets", strings.Join(pattern[:len(pattern)-1], "/"))
			default:
				return errors.New("unknown platform")
			}
			log.Default().Println(filePath, dir)

			err := os.MkdirAll(dir, 0755)
			if err != nil {
				return err
			}
			f, err := os.Create(filePath)
			if err != nil {
				return err
			}
			defer f.Close()
			fmt.Printf("%s/%s %s\n", style.ChangedString(secretsPath+"/"+secretManager.Platform()), style.ChangedString(args[0]), style.ChangedString("created locally"))
			if useEditor {
				if err = utils.OpenWithEditor(args, secretsPath+"/"+secretManager.Platform()); err != nil {
					return err
				}
			}
			return nil
		},
	}
}

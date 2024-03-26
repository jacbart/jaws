package main

import (
	"fmt"

	"github.com/jacbart/jaws/utils"
	"github.com/spf13/cobra"
)

func PathCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "path",
		Short:   "prints path to secrets folder and will create the path if it does not exist",
		Example: "cd $(jaws path)",
		RunE: func(cmd *cobra.Command, args []string) error {
			return utils.EnsurePath(secretsPath)
		},
	}
}

func PathCommandCmd() *cobra.Command {
	return &cobra.Command{
		Use:     "command",
		Short:   "prints out the shell function that lets jaws-cd work properly",
		Example: "source <(jaws path command)",
		Run: func(cmd *cobra.Command, args []string) {
			shCommand := `function jaws-cd() {
				if [[ $(pwd) == $(jaws path) ]]; then
				  popd;
				else
				  pushd $(jaws path);
				fi
			  }
			  
			  alias kd=jaws-cd`
			fmt.Println(shCommand)
		},
	}
}

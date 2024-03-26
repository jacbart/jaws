package secretsmanager

import (
	"context"
	"errors"
	"fmt"
	"log"
	"os"

	"github.com/jacbart/jaws/integration/gcp"
	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
)

func (g GCPManager) Push(secretsPath string, createPrompt bool) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	service, err := LoadGCPClient(&g, ctx)
	if err != nil {
		return err
	}

	sIDs, err := utils.PullSecretNames(secretsPath)
	if err != nil {
		return err
	}
	log.Default().Println(sIDs)

	l := len(sIDs)
	var secretUpdate []byte
	for i := 0; i < l; i++ {
		f := secretsPath + "/" + g.DefaultProject + "/secrets/" + sIDs[i]
		if _, err := os.Stat(f); err == nil {
			secretUpdate, err = os.ReadFile(f)
			if err != nil {
				return err
			}

			// check if there is an update and only push if there is one
			shouldSecretUpdate, err := gcp.CheckIfUpdate(ctx, service, g.DefaultProject, sIDs[i], string(secretUpdate))
			if err != nil {
				return err
			}

			// handler for updating or creating a new secret
			if shouldSecretUpdate {
				if err = gcp.HandleUpdateCreate(ctx, service, g.DefaultProject, sIDs[i], string(secretUpdate), createPrompt); err != nil {
					return err
				}
			} else {
				fmt.Printf("%s %s\n", g.DefaultProject+"/secrets/"+sIDs[i], style.InfoString("skipped"))
			}

		} else if errors.Is(err, os.ErrNotExist) {
			log.Default().Println(f, "does not exist")
			continue
		} else {
			return err
		}

	}
	return nil
}

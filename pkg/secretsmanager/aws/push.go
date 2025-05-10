package aws

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/jacbart/jaws/integration/aws"
	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
)

// AWS Manager Push
func (m Manager) Push(secretsPath string, createPrompt bool) error {
	log.Default().Println("searching", secretsPath, "for secrets to push")
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := m.LoadClient(ctx)
	if err != nil {
		return err
	}

	sIds, err := utils.PullSecretNames(secretsPath)
	if err != nil {
		return err
	}
	log.Default().Println("secrets found:", sIds)

	l := len(sIds)
	var secretUpdate []byte
	for i := range l {
		log.Default().Println("reading", secretsPath+"/"+sIds[i])
		secretUpdate, err = os.ReadFile(secretsPath + "/" + sIds[i])
		if err != nil {
			return err
		}
		shouldSecretUpdate, err := aws.CheckIfUpdate(ctx, client, sIds[i], string(secretUpdate))
		if err != nil {
			return nil
		}
		if shouldSecretUpdate {
			if err = aws.HandleUpdateCreate(ctx, client, sIds[i], string(secretUpdate), createPrompt); err != nil {
				return err
			}
		} else {
			fmt.Printf("%s %s\n", sIds[i], style.InfoString("skipped"))
		}
	}
	return nil
}

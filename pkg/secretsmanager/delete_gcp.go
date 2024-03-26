package secretsmanager

import (
	"context"
	"fmt"

	"github.com/jacbart/jaws/utils/style"
)

// GCPManager Delete takes a slice of Secret and deletes them from the gcp secrets manager
func (g GCPManager) Delete() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	service, err := LoadGCPClient(&g, ctx)
	if err != nil {
		return err
	}

	for _, secret := range g.Secrets {
		deleteCall := service.Delete(secret.ID)
		_, err = deleteCall.Do()
		if err != nil {
			return err
		}
		fmt.Printf("%s %s\n", secret.ID, style.FailureString("deleted"))
	}
	return nil
}

func (g GCPManager) CancelDelete() error {
	fmt.Println("use the delete command for GCP")
	return nil
}

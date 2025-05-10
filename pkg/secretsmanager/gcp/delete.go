package gcp

import (
	"context"
	"fmt"

	"github.com/jacbart/jaws/utils/style"
)

// GCP Manager Delete takes a slice of Secret and deletes them from the gcp secrets manager
func (m Manager) Delete() error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	service, err := m.LoadClient(ctx)
	if err != nil {
		return err
	}

	for _, secret := range m.Secrets {
		deleteCall := service.Delete(secret.ID)
		_, err = deleteCall.Do()
		if err != nil {
			return err
		}
		fmt.Printf("%s %s\n", secret.ID, style.FailureString("deleted"))
	}
	return nil
}

func (m Manager) CancelDelete() error {
	fmt.Println("use the delete command for GCP")
	return nil
}

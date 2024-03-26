package secretsmanager

import (
	"fmt"
)

// PrintSecrets prints a slice of Secrets
func PrintSecrets(Secrets []Secret) {
	for _, s := range Secrets {
		fmt.Println(s.Content)
	}
}

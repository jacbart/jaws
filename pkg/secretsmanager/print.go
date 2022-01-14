package secretsmanager

import (
	"fmt"

	"github.com/fatih/color"
)

func CleanPrintSecrets(Secrets []Secret) {
	for _, s := range Secrets {
		fmt.Println(s.ID)
		fmt.Println(s.Content)
	}
}

func FormatPrintSecret(Secrets []Secret) {
	for _, s := range Secrets {
		color.Magenta(s.ID)
		color.HiGreen(s.Content)
	}
}

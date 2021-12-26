package utils

import (
	"context"
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"strings"

	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/jacbart/fidelius-charm/internal/aws"
)

func PrintListFZF(ctx context.Context, client *secretsmanager.Client) ([]string, error) {
	var l int

	filtered, err := withFilter("fzf -m", func(in io.WriteCloser) {
		listSecretsOutput, err := aws.GetSecretsList(ctx, client, nil)
		if err != nil {
			log.Fatalf("%v", err)
		}
		l = len(listSecretsOutput.SecretList)
		for i := 0; i < l; i++ {
			fmt.Fprintln(in, *listSecretsOutput.SecretList[i].Name)
		}
		for listSecretsOutput.NextToken != nil {
			listSecretsOutput, err = aws.GetSecretsList(ctx, client, listSecretsOutput.NextToken)
			if err != nil {
				log.Fatalf("%v", err)
			}
			l = len(listSecretsOutput.SecretList)
			for i := 0; i < l; i++ {
				fmt.Fprintln(in, *listSecretsOutput.SecretList[i].Name)
			}
		}
	})
	if err != nil {
		return []string{}, err
	}
	return filtered, nil
}

func withFilter(command string, input func(in io.WriteCloser)) ([]string, error) {
	shell := os.Getenv("SHELL")
	if len(shell) == 0 {
		shell = "sh"
	}
	cmd := exec.Command(shell, "-c", command)
	cmd.Stderr = os.Stderr
	in, _ := cmd.StdinPipe()
	go func() {
		input(in)
		in.Close()
	}()
	result, err := cmd.Output()
	if err != nil {
		return []string{}, err
	}
	return strings.Split(string(result), "\n"), nil
}

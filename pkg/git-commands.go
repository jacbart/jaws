package firm

import (
	"os"
	"os/exec"
)

func GitDiff(secretsPath string) error {
	c := exec.Command("git", "diff")
	c.Dir = secretsPath
	c.Stderr = os.Stderr
	c.Stdout = os.Stdout
	c.Run()
	return nil
}

func GitStatus(secretsPath string) error {
	c := exec.Command("git", "status")
	c.Dir = secretsPath
	c.Stderr = os.Stderr
	c.Stdout = os.Stdout
	c.Run()
	return nil
}

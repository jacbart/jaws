package manager

import (
	"fmt"
	"os"
	"strings"
)

func DownloadSecret(secretID string, secretString string, secretsPath string) error {
	pattern := strings.Split(secretID, "/")
	filePath := fmt.Sprintf("%s/%s", secretsPath, secretID)
	dir := fmt.Sprintf("%s/%s", secretsPath, strings.Join(pattern[:len(pattern)-1], "/"))
	err := os.MkdirAll(dir, 0755)
	if err != nil {
		return err
	}
	f, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer f.Close()

	_, err = f.WriteString(secretString)
	if err != nil {
		return err
	}
	err = f.Close()
	if err != nil {
		return err
	}
	return nil
}

package utils

import (
	"os"
	"path/filepath"
	"strings"
)

// PullSecretNames walks the secretsPath and creates a list of secrets that are in there.
// Returns a list of local secrets and an error
func PullSecretNames(secretsPath string) ([]string, error) {
	var secretNames []string
	err := filepath.WalkDir(secretsPath,
		func(path string, d os.DirEntry, err error) error {
			if err != nil {
				return err
			}
			info, err := os.Lstat(path)
			if err != nil {
				return err
			}
			if !info.IsDir() {
				var secretID string
				if strings.HasSuffix(secretsPath, "gcp") || strings.HasSuffix(secretsPath, "aws") {
					if strings.Contains(path, ".git") {
						return nil
					}
					secretID = strings.TrimPrefix(path, secretsPath+"/")
				} else {
					secretsPath = strings.TrimPrefix(secretsPath, "./")
					secretID = strings.TrimPrefix(path, secretsPath+"/")
				}
				if !strings.HasPrefix(secretID, ".") {
					secretNames = append(secretNames, secretID)
				}
			}
			return nil
		})
	if err != nil {
		return []string{}, err
	}
	return secretNames, nil
}

package envmanager

import (
	"encoding/json"
	"log"
	"os"
	"path/filepath"
	"strings"

	"gopkg.in/yaml.v2"
)

// checkForEnvFile
func checkForEnvFile(file string) error {
	if _, err := os.Stat(file); err != nil {
		return &NoEnvFileFound{File: file}
	}
	return nil
}

func isJSON(s string) bool {
	var js map[string]interface{}
	return json.Unmarshal([]byte(s), &js) == nil
}

func isYAML(s string) bool {
	var yml map[string]interface{}
	return yaml.Unmarshal([]byte(s), &yml) == nil
}

// AddEnvConfig
func (e *EnvConfig) AddEnvConfig(file string) error {
	// list all files in current directory
	err := checkForEnvFile(file)
	if err != nil {
		return err
	}

	// check if file is a directory
	info, _ := os.Stat(file)
	if info.IsDir() {
		return &EnvIsDir{Path: file}
	}

	e.Env = append(e.Env, &EnvHCL{
		ConfigFile: file,
		Prepared:   false,
		Processed:  false,
	})
	return nil
}

// SearchDir
func (e *EnvConfig) SearchDir(dir string) error {
	// list all files in current directory
	files, err := os.ReadDir(dir)
	if err != nil {
		log.Default().Fatal(err)
	}
	// search for file ending in .jaws
	for _, file := range files {
		if strings.Contains(file.Name(), ".jaws") {
			info, _ := os.Stat(file.Name())
			if !info.IsDir() {
				e.Env = append(e.Env, &EnvHCL{
					ConfigFile: file.Name(),
					Prepared:   false,
					Processed:  false,
				})
			}
		}
	}
	return nil
}

// RecursiveSearchDir
func (e *EnvConfig) RecursiveSearchDir(dir string) error {
	err := filepath.Walk(dir,
		func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if strings.HasSuffix(path, ".jaws") {
				info, _ := os.Stat(path)
				if !info.IsDir() {
					e.Env = append(e.Env, &EnvHCL{
						ConfigFile: path,
						Prepared:   false,
						Processed:  false,
					})
				}
			}
			return nil
		})
	if err != nil {
		return err
	}
	return nil
}

package envmanager

import (
	"bytes"
	"errors"
	"fmt"
	"io"
	"os"
	"strings"
	"time"

	"github.com/jacbart/jaws/utils"
	"github.com/jacbart/jaws/utils/style"
)

func (e *EnvConfig) Write() error {
	for _, env := range e.Env {
		var existingOutFile bool
		var err error
		isDiff := false
		var writer io.Writer

		// Create Buffer as Writer
		buf := bytes.NewBuffer(make([]byte, 0))
		// Read from decrypt to buf
		_, err = buf.ReadFrom(env.Reader)
		if err != nil {
			return err
		}
		if env.OutFile != "-" {
			outfileStat, err := os.Stat(env.OutFile)
			if err == nil { // the out file already exists
				existingOutFile = true
				if e.Options.Diff {
					if e.Options.Overwrite {
						e.Options.UnsafeMode = true
					}

					b, err := os.ReadFile(env.OutFile)
					if err != nil {
						return err
					}

					if isDiff, err = utils.CompareStrings(string(b[:]), buf.String(), true); err != nil {
						return err
					}
					if !isDiff {
						fmt.Printf("no changes detected for %s\nquitting...\n", env.OutFile)
						return nil
					}
				} else {
					if e.Options.Overwrite {
						e.Options.UnsafeMode = true
					} else if !e.Options.UnsafeMode {
						e.Options.Overwrite = true
					}

					b, err := os.ReadFile(env.OutFile)
					if err != nil {
						return err
					}
					if isDiff, err = utils.CompareStrings(string(b[:]), buf.String(), false); err != nil {
						return err
					}
					if !isDiff {
						fmt.Printf("no changes detected for %s\nquitting...\n", env.OutFile)
						return nil
					}
				}

				if !e.Options.Overwrite {
					var userResponse string
					if e.Options.UnsafeMode {
						fmt.Printf("%s '%s'? [y/N] ", style.FailureString("overwrite"), env.OutFile)
					} else {
						fmt.Printf("create new '%s' and backup '%s'? [Y/n] ", env.OutFile, env.OutFile)
					}
					fmt.Scanln(&userResponse)

					userResponse = strings.TrimSpace(userResponse)
					userResponse = strings.ToLower(userResponse)

					if userResponse == "y" || userResponse == "yes" {
						if e.Options.UnsafeMode {
							e.Options.Overwrite = true
						}
					} else {
						fmt.Println("quitting...")
						return nil
					}
				}
			} else if errors.Is(err, os.ErrNotExist) { // the file does not exist, create it without conflict
				existingOutFile = false
			} else { // other known error
				return err
			}
			if existingOutFile {
				if !e.Options.UnsafeMode {
					env.OutFile = strings.TrimPrefix(env.OutFile, "./")
					backupEnvName := outfileStat.ModTime().Format(time.RFC3339) + "-" + env.OutFile
					if err = os.Rename(env.OutFile, backupEnvName); err != nil {
						return err
					}
					fmt.Printf("backed up %s to %s\n", env.OutFile, backupEnvName)
					e.Options.Overwrite = false
				}
				if e.Options.Overwrite {
					err := os.Remove(env.OutFile)
					if err != nil {
						return err
					}
				}
				fmt.Printf("%s %s\n", style.InfoString("writing"), env.OutFile)
				f, err := os.Create(env.OutFile)
				if err != nil {
					return err
				}
				defer f.Close()
				writer = f
			} else {
				fmt.Printf("%s %s\n", style.InfoString("creating"), env.OutFile)
				f, err := os.Create(env.OutFile)
				if err != nil {
					return err
				}
				defer f.Close()
				writer = f
			}
		} else {
			writer = os.Stdout
		}

		if buf.String() != "" {
			_, err = io.Copy(writer, buf)
			if err != nil {
				return err
			}
		}
	}
	return nil
}

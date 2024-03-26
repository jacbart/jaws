package utils

import (
	"archive/tar"
	"bufio"
	"compress/gzip"
	"fmt"
	"io"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
)

// OpenWithEditor will open a list of files with whatever the env var EDITIOR is set to
func OpenWithEditor(files []string, path string) error {
	var filesList []string
	if len(files) == 0 {
		return fmt.Errorf("no files selected")
	}
	for _, file := range files {
		filesList = append(filesList, fmt.Sprintf("%s/%s", path, file))
	}
	editor, present := os.LookupEnv("EDITOR")
	if !present {
		fmt.Printf("set EDITOR environment varible in order to not see this again\n")
		fmt.Printf("Enter editor: ")
		var newEditor string
		fmt.Scanln(&newEditor)
		editor = newEditor
	}

	editCmd := exec.Command(editor, filesList...)
	editCmd.Stdin = os.Stdin
	editCmd.Stdout = os.Stdout
	editCmd.Stderr = os.Stderr
	if err := editCmd.Run(); err != nil {
		return fmt.Errorf("opening one of or all %s with editor: %w", filesList[:], err)
	}
	return nil
}

// RunCommand takes an interpreter with a list of arguments and runs the command returning the output as a string and an error
func RunCommand(interpreter string, args []string) (string, error) {
	cmd := exec.Command(interpreter, args...)
	b, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("failed to run: %s %s %w", interpreter, args[:], err)
	}
	return string(b), nil
}

// ShowFile will displays the contents of file
func ShowFile(file string) error {
	f, err := os.Open(file)
	if err != nil {
		return err
	}
	defer func() {
		if err = f.Close(); err != nil {
			log.Default().Fatal(err)
		}
	}()

	scanner := bufio.NewScanner(f)

	for scanner.Scan() {
		fmt.Println(scanner.Text())
	}
	return nil
}

// CheckIfPrefix returns true of input string ends with a / or *
func CheckIfPrefix(input string) bool {
	lastChar := input[len(input)-1:]
	if lastChar == "*" || lastChar == "/" {
		return true
	}
	return false
}

// FormatPrefixString returns string with /* as the last two characters
func FormatPrefixString(prefix string) string {
	lastChar := prefix[len(prefix)-1:]
	switch lastChar {
	case "*":
		secondLastChar := prefix[len(prefix)-2 : len(prefix)-1]
		if secondLastChar != "/" {
			prefix = strings.TrimSuffix(prefix, "*")
			prefix = fmt.Sprintf("%s/*", prefix)
		}
	case "/":
		prefix = fmt.Sprintf("%s*", prefix)
	default:
		prefix = fmt.Sprintf("%s/*", prefix)
	}
	return prefix
}

// Untar takes a destination path and a reader; a tar reader loops over the tarfile
// creating the file structure at 'dst' along the way, and writing any files
func Untar(dst string, r io.Reader) error {
	gzr, err := gzip.NewReader(r)
	if err != nil {
		return err
	}
	defer gzr.Close()

	tr := tar.NewReader(gzr)

	for {
		header, err := tr.Next()

		switch {

		// if no more files are found return
		case err == io.EOF:
			return nil

		// return any other error
		case err != nil:
			return err

		// if the header is nil, just skip it (not sure how this happens)
		case header == nil:
			continue
		}

		// the target location where the dir/file should be created
		target := filepath.Join(dst, header.Name)

		// check the file type
		switch header.Typeflag {

		// if its a dir and it doesn't exist create it
		case tar.TypeDir:
			if _, err := os.Stat(target); err != nil {
				if err := os.MkdirAll(target, 0755); err != nil {
					return err
				}
			}

		// if it's a file create it
		case tar.TypeReg:
			f, err := os.OpenFile(target, os.O_CREATE|os.O_RDWR, os.FileMode(header.Mode))
			if err != nil {
				return err
			}

			// copy over contents
			if _, err := io.Copy(f, tr); err != nil {
				return err
			}

			// manually close here after each file operation; defering would cause each file close
			// to wait until all operations have completed.
			f.Close()
		}
	}
}

// DownloadSecret - Creates the directory path using the secrets name and the delimiter set ususally to /, then writes the secret the final file
func DownloadSecret(secretID string, secretString string, secretsPath string, delimiter string) error {
	pattern := strings.Split(secretID, delimiter)
	filePath := fmt.Sprintf("%s%s%s", secretsPath, delimiter, secretID)
	dir := fmt.Sprintf("%s%s%s", secretsPath, delimiter, strings.Join(pattern[:len(pattern)-1], "/"))
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

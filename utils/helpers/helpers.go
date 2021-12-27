package helpers

import (
	"fmt"
	"os"
	"os/exec"
	"time"

	"github.com/fatih/color"
	"github.com/go-git/go-git/v5"
	"github.com/go-git/go-git/v5/plumbing/object"
)

func OpenEditor(secretsList []string) error {
	if len(secretsList) == 0 {
		return fmt.Errorf("no secrets selected")
	}
	editor, present := os.LookupEnv("EDITOR")
	if !present {
		fmt.Printf("set EDITOR environment varible in order to not see this again\n")
		fmt.Printf("Enter editor: ")
		var newEditor string
		fmt.Scanln(&newEditor)
		editor = newEditor
	}

	editCmd := exec.Command(editor, secretsList...)
	editCmd.Stdin = os.Stdin
	editCmd.Stdout = os.Stdout
	editCmd.Stderr = os.Stderr
	if err := editCmd.Run(); err != nil {
		return fmt.Errorf("opening secret with editor: %w", err)
	}
	return nil
}

func CheckIfGitRepo(path string, shouldWarn bool) bool {
	_, err := os.Stat(fmt.Sprintf("%s/.git", path))
	if os.IsNotExist(err) {
		return false
	}

	if shouldWarn {
		gitRepoWarning()
	}
	return true
}

func gitRepoWarning() {
	color.Yellow("CAUTION the directory you are working in is a git repo")
	color.Yellow("        !!  DO NOT COMMIT ANY SECRETS  !!")
	color.Cyan("recommend putting 'secrets' into your .gitignore file")
}

func GitControlSecrets(secretIDs []string, secretsPath string) error {
	isRepo := CheckIfGitRepo(secretsPath, false)
	var repo *git.Repository
	var err error
	if isRepo {
		repo, err = git.PlainOpen(secretsPath)
		if err != nil {
			return err
		}
	} else {
		repo, err = git.PlainInit(secretsPath, false)
		if err != nil {
			return err
		}
	}
	w, err := repo.Worktree()
	if err != nil {
		return err
	}

	l := len(secretIDs)
	var addOptions *git.AddOptions

	for i := 0; i < l-1; i++ {
		addOptions = &git.AddOptions{
			All:  false,
			Path: secretIDs[i],
		}
		if err = w.AddWithOptions(addOptions); err != nil {
			return err
		}
	}

	commitOptions := &git.CommitOptions{
		All: false,
		Author: &object.Signature{
			Name:  "Fidelius Charm",
			Email: "firm@local.com",
			When:  time.Now(),
		},
	}
	_, err = w.Commit(fmt.Sprintf("firm commit %v", time.Now()), commitOptions)
	if err != nil {
		return err
	}
	return nil
}

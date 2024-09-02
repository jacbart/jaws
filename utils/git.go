package utils

import (
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"log"
	"net/http"
	"os"
	"os/exec"
	"path"
	"regexp"
	"runtime"
	"strings"
	"time"

	"github.com/Masterminds/semver"
	"github.com/go-git/go-git/v5"
	"github.com/go-git/go-git/v5/plumbing/object"
	"github.com/google/go-github/github"
	"github.com/jacbart/jaws/utils/style"
	"golang.org/x/oauth2"
	"golang.org/x/text/cases"
	"golang.org/x/text/language"
)

const (
	gitEmail         = "secrets.manager@jaws.cli"
	gitUser          = "Secrets Manager"
	gitInitCommitMsg = "initial commit of secrets"
	gitOrg           = "jacbart"
	gitRepo          = "jaws"
	gitRepoPath      = "jacbart/jaws"
)

// GitDiff - Replace me with golang version
func GitDiff(secretsPath string) error {
	c := exec.Command("git", "diff")
	c.Dir = secretsPath
	c.Stderr = os.Stderr
	c.Stdout = os.Stdout
	c.Run()
	return nil
}

// GitStatus - runs git status on the secrets folder - Replace me with golang version
func GitStatus(path string) error {
	c := exec.Command("git", "status")
	c.Dir = path
	c.Stderr = os.Stderr
	c.Stdout = os.Stdout
	c.Run()
	return nil
}

// CheckIfGitRepo - runs git diff on the secrets folder - checks path for a .git folder and returns true if found
func CheckIfGitRepo(path string, shouldWarn bool) bool {
	_, err := os.Stat(fmt.Sprintf("%s/.git", path))
	if os.IsNotExist(err) {
		return false
	}

	if shouldWarn {
		repoWarningMessage(fmt.Sprintf("%s/secrets", path))
	}
	return true
}

// repoWarningMessage - Warning message for when person is working in a git repo
func repoWarningMessage(path string) {
	fmt.Println(style.InfoString("CAUTION the directory you are working in is a git repo"))
	fmt.Println(style.InfoString("        !!  DO NOT COMMIT ANY SECRETS  !!"))
	fmt.Println(style.InfoString("recommend putting"), style.InfoString(path), style.InfoString("into your .gitignore file"))
}

// GitControlSecrets - creates a local git repo and commits the initially downloaded secrets
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
			Name:  gitUser,
			Email: gitEmail,
			When:  time.Now(),
		},
	}
	_, err = w.Commit(fmt.Sprintf("%s %v", gitInitCommitMsg, time.Now()), commitOptions)
	if err != nil {
		return err
	}
	return nil
}

// GitCheckForUpdate returns an error and a semver containing the updated tag if it exists
func GitCheckForUpdate(tc *http.Client, parentCtx context.Context, currentVersion string) (*semver.Version, error) {
	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	// create github client
	client := github.NewClient(tc)
	// pull all releases
	releases, _, err := client.Repositories.ListReleases(ctx, gitOrg, gitRepo, nil)
	if err != nil {
		return nil, err
	}

	// set current semver
	cv, err := semver.NewVersion(currentVersion)
	if err != nil {
		return nil, err
	}

	// new version var
	var nv *semver.Version
	// set new version to current version
	nv = cv

	// loop over all releases
	for _, r := range releases {
		// get semver of release
		v, err := semver.NewVersion(r.GetTagName())
		if err != nil {
			return nil, err
		}

		// if release is greater than nv set as new version
		if v.GreaterThan(nv) {
			nv = v
		}
		log.Default().Printf("release: %s\n", v.String())
	}

	if nv.Equal(cv) {
		return nil, nil
	} else {
		return nv, nil
	}
}

// GitLatestRelease downloads the latest version of jaws if there is a newer version
func GitLatestRelease(currentVersion, token string) error {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// static token for github oauth2
	ts := oauth2.StaticTokenSource(
		&oauth2.Token{AccessToken: token},
	)
	// http client using oauth2
	tc := oauth2.NewClient(ctx, ts)

	nv, err := GitCheckForUpdate(tc, ctx, currentVersion)
	if err != nil {
		return err
	}
	// if latest detected released version is greater than current version
	if nv != nil {
		latestTag := nv.String()
		latestUrl := fmt.Sprintf("https://api.github.com/repos/%s/releases/tags/v%s", gitRepoPath, latestTag)
		fmt.Printf("new version %s -> %s\n", style.ChangedString(currentVersion), style.SuccessString(latestTag))

		res, err := tc.Get(latestUrl)
		if err != nil {
			return err
		}
		defer res.Body.Close()

		bodyText, err := io.ReadAll(res.Body)
		if err != nil {
			return err
		}

		// prepare result
		result := make(map[string]interface{})
		json.Unmarshal(bodyText, &result)

		results := make([]interface{}, 0)

		for _, asset := range result["assets"].([]interface{}) {
			results = append(results, asset.(map[string]interface{})["id"])
		}

		// get runtime OS and capitalize the first letter
		osCase := cases.Title(language.English)
		osName := osCase.String(runtime.GOOS)

		// get the runtime archeticture
		arch := runtime.GOARCH

		// search filter for downloading the right release
		dlAssetFilter := fmt.Sprintf("%s_%s", osName, arch)
		log.Default().Printf("release filter: %s\n", dlAssetFilter)

		// download assets
		c := make(chan int)
		for _, res := range results {
			go downloadGitAsset(res.(float64), token, dlAssetFilter, c)
		}
		// wait for downloads to finish
		for i := 0; i < len(results); i++ {
			<-c
		}

		// list all files in current directory
		files, err := os.ReadDir(".")
		if err != nil {
			return err
		}

		// search for downloaded tar.gz containing the updated binary
		tarFile := ""
		for _, file := range files {
			if strings.Contains(file.Name(), dlAssetFilter) {
				tarFile = file.Name()
			}
		}
		if tarFile == "" {
			return errors.New("tar.gz file not found after download")
		}

		dir := os.TempDir()
		// open downlaoded tar.gz file
		r, err := os.Open(tarFile)
		if err != nil {
			return err
		}

		// un-tar.gz the downloaded file
		err = Untar(dir, r)
		if err != nil {
			return err
		}

		// test if the right binary was downlaoded
		dlVersion, err := RunCommand(fmt.Sprintf("%s/jaws", dir), []string{"version", "--short"})
		if err != nil {
			return err
		}
		dlVersion = strings.TrimSuffix(dlVersion, "\n")
		log.Default().Printf("version %s downloaded\n", style.SuccessString(dlVersion))

		// clean up tar file
		err = os.Remove(tarFile)
		if err != nil {
			return err
		}

		// get current running jaws location
		e, err := os.Executable()
		if err != nil {
			return err
		}

		// backup old jaws version
		err = os.Rename(e, fmt.Sprintf("%s/jaws.old", path.Dir(e)))
		if err != nil {
			return err
		}

		// move file to currently install location
		err = os.Rename(fmt.Sprintf("%s/jaws", dir), fmt.Sprintf("%s/jaws", path.Dir(e)))
		if err != nil {
			return err
		}

		fmt.Printf("%s: %s\n", style.InfoString("update installed"), style.SuccessString(dlVersion))
	} else {
		fmt.Printf("%s: running latest or newer\n", style.InfoString("no updates"))
	}
	return nil
}

// downloadGitAsset downloads assest from github, use channel to manage concurrency
func downloadGitAsset(id float64, token, dlAssetFilter string, c chan int) {
	defer func() { c <- 1 }()
	url := fmt.Sprintf("https://api.github.com/repos/%s/releases/assets/%.0f", gitRepoPath, id)

	req, _ := http.NewRequest("GET", url, nil)
	req.Header.Add("Authorization", fmt.Sprintf("token %s", token))
	req.Header.Add("User-Agent", "jaws-update-client")

	req.Header.Add("Accept", "application/octet-stream")

	client := http.Client{}
	resp, _ := client.Do(req)

	disp := resp.Header.Get("Content-disposition")
	re := regexp.MustCompile(`filename=(.+)`)
	matches := re.FindAllStringSubmatch(disp, -1)

	if len(matches) == 0 || len(matches[0]) == 0 {
		log.Default().Fatalf("%v \n-------\n %v", matches, resp.Header)
	}

	disp = matches[0][1]
	if strings.Contains(disp, dlAssetFilter) {
		f, err := os.OpenFile(disp, os.O_WRONLY|os.O_CREATE|os.O_TRUNC, 0664)
		if err != nil {
			log.Default().Fatal(err)
		}
		defer f.Close()

		b := make([]byte, 4096)
		var i int

		for err == nil {
			i, err = resp.Body.Read(b)
			f.Write(b[:i])
		}
	}
}

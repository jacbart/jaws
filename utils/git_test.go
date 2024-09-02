//go:build unit

package utils_test

import (
	"context"
	"os"
	"testing"

	"github.com/Masterminds/semver"
	"github.com/jacbart/jaws/utils"
	"golang.org/x/oauth2"
)

var (
	token          = ""
	currentVersion = "1.0.0"
)

func TestGitCheckForUpdate(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	if t, present := os.LookupEnv("GH_TOKEN"); present {
		token = t
	}

	if token == "" {
		t.Errorf("Need github token to run this test, set GH_TOKEN env var")
	}

	// static token for github oauth2
	ts := oauth2.StaticTokenSource(
		&oauth2.Token{AccessToken: token},
	)
	// http client using oauth2
	tc := oauth2.NewClient(ctx, ts)

	cv, err := semver.NewVersion(currentVersion)
	if err != nil {
		t.Error(err)
	}

	nv, err := utils.GitCheckForUpdate(tc, ctx, currentVersion)
	if err != nil {
		t.Error(err)
	}

	if nv == nil {
		t.Errorf("GitCheckForUpdate Failed: no new version found")
	} else {
		if cv.Equal(nv) {
			t.Errorf("GitCheckForUpdate Failed: return same version expect newer")
		} else if cv.GreaterThan(nv) {
			t.Errorf("GitCheckForUpdate Failed: returned older version, expected a newer version")
		}
	}
}

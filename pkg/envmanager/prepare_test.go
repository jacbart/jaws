//go:build unit

package envmanager

import (
	"fmt"
	"testing"

	"github.com/jacbart/jaws/pkg/secretsmanager"
)

func TestPrepare(t *testing.T) {
	env := InitEnv(nil)

	// err := env.AddEnvConfig("testdata/basic-aws-config")
	// if err != nil {
	// 	t.Error(err)
	// }

	err := env.AddEnvConfig("testdata/basic-local-config")
	if err != nil {
		t.Error(err)
	}

	fmt.Println("starting prepare")
	err = env.Prepare()
	if err != nil {
		t.Errorf("Load config file Failed %v", err)
	}
	fmt.Println(env.SecretIDs)
	var secrets []secretsmanager.Secret
	for _, sID := range env.SecretIDs {
		secrets = append(secrets, secretsmanager.Secret{
			ID:      sID,
			Content: "test",
		})
	}

	for _, e := range env.Env {
		err = e.Process(secrets)
		if err != nil {
			t.Error(err)
		}
	}

	err = env.Write()
	if err != nil {
		t.Error(err)
	}

	// if env.Env["testdata/basic-aws-config"].Filter != "testing/jaws/prefix/*" {
	// 	t.Errorf("Loading prefix failed, expected testing/jaws/prefix/* got %s", env.ConfigFile["testdata/basic-aws-config"].Filter)
	// }

	// if env.ConfigFile["testdata/basic-local-config"].Filter != "" {
	// 	t.Errorf("Loading prefix failed, expected nothing got %s", env.ConfigFile["testdata/basic-aws-config"].Filter)
	// }

}

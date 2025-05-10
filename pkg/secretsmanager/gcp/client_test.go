//go:build unit

package secretsmanager_test

// func TestLoadGCPClient(t *testing.T) {

// 	// g := secretsmanager.InitManager("gcp")

// 	g := secretsmanager.GCPManager{
// 		ProfileLabel: "Test GCP",
// 		CredFile:     "./testdata/gcp-service-account.json",
// 	}

// 	_, err := secretsmanager.LoadGCPClient(&g, context.Background())
// 	if err != nil {
// 		t.Error(err)
// 	}

// 	// secretsFuzzyList, err := g.FuzzyFind(context.Background(), "")
// 	// if err != nil {
// 	// 	t.Error(err)
// 	// }
// 	// fmt.Println(secretsFuzzyList)

// 	// err := g.Push("secrets/gcp", false)
// 	// if err != nil {
// 	// 	t.Error(err)
// 	// }

// 	// secretsList := g.ListAll("")
// 	// fmt.Println(secretsList)

// 	// err = g.Rollback()

// 	// shouldUpdate, err := gcp.CheckIfUpdate(context.Background(), service, g.DefaultProject, "testing_key", "")
// 	// if err != nil {
// 	// 	t.Error(err)
// 	// }
// 	// fmt.Println(shouldUpdate)

// 	// secrets, err := g.Pull([]string{"testing_key"})
// 	if err != nil {
// 		t.Error(err)
// 	}
// 	fmt.Println(g.DefaultProject)
// 	// fmt.Println(secrets)
// }

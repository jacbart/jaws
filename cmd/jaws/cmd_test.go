//go:build integration

package main_test

import (
	"bytes"
	"fmt"
	"io"
	"testing"

	main "github.com/jacbart/jaws/cmd/jaws"
	"github.com/spf13/cobra"
)

func TestAddCmd(t *testing.T) {
	cobra.OnInitialize(main.InitConfig)
	main.Commands()
	main.Flags()

	cmd := main.RootCmd()
	b := bytes.NewBufferString("")
	cmd.SetOut(b)
	cmd.SetArgs([]string{"add", "test"})
	cmd.Execute()
	out, err := io.ReadAll(b)
	if err != nil {
		t.Fatal(err)
	}
	fmt.Println(string(out))
	// if string(out) != "hi" {
	// 	t.Fatalf("expected \"%s\" got \"%s\"", "hi", string(out))
	// }
}

//go:build unit

package tui_test

import (
	"fmt"
	"testing"

	"github.com/jacbart/jaws/utils/tui"
)

func TestSelectorTUI(t *testing.T) {
	choices := []string{"Choose me", "choose me 2", "blah", "blah again", "another item", "and another one", "Plus...another one", ":)", ":("}
	s, err := tui.FuzzySelectorTUI(choices)
	if err != nil {
		t.Error(err)
	}
	fmt.Println(s)
}

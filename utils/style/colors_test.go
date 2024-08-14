//go:build unit

package style_test

import (
	"fmt"
	"testing"

	"github.com/jacbart/jaws/utils/style"
)

func TestColors(t *testing.T) {
	fmt.Println(style.SuccessString("Success"))
	fmt.Println(style.FailureString("Failed"))
	fmt.Println(style.WarningString("Warning"))
	fmt.Println(style.InfoString("Info"))
	fmt.Println(style.ChangedString("Something Changed"))
}

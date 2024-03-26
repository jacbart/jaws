package utils

import (
	"bufio"
	"fmt"
	"strings"

	"github.com/jacbart/jaws/utils/style"
)

func CompareStrings(str1, str2 string, print bool) (bool, error) {
	isDiff := false
	scanner1 := bufio.NewScanner(strings.NewReader(str1))
	scanner2 := bufio.NewScanner(strings.NewReader(str2))
	comp1 := ""
	comp2 := ""
	eof1 := true
	eof2 := true
	for {
		if !eof1 && !eof2 {
			break
		}
		if eof1 {
			eof1 = scanner1.Scan()
		}
		if eof2 {
			eof2 = scanner2.Scan()
		}

		if scanner1.Text() != scanner2.Text() {
			isDiff = true
			if scanner1.Text() != "" {
				if comp1 == "" {
					comp1 = style.FailureString("-") + style.FailureString(scanner1.Text())
				} else {
					comp1 = comp1 + "\n" + style.FailureString("-") + style.FailureString(scanner1.Text())
				}
			}

			if scanner2.Text() != "" {
				if comp2 == "" {
					comp2 = style.SuccessString("+") + style.SuccessString(scanner2.Text())
				} else {
					comp2 = comp2 + "\n" + style.SuccessString("+") + style.SuccessString(scanner2.Text())
				}
			}
		} else if print {
			if comp1 != "" {
				fmt.Printf("%s\n", comp1)
				comp1 = ""
			}
			if comp2 != "" {
				fmt.Printf("%s\n", comp2)
				comp2 = ""
			}
			if eof1 {
				fmt.Printf("%s\n", scanner1.Text())
			} else if eof2 {
				fmt.Printf("%s\n", scanner2.Text())
			}
		}
	}
	if print {
		if comp1 != "" {
			fmt.Printf("%s\n", comp1)
		}
		if comp2 != "" {
			fmt.Printf("%s\n", comp2)
		}
	}

	if err := scanner1.Err(); err != nil {
		return false, err
	}
	if err := scanner2.Err(); err != nil {
		return false, err
	}
	return isDiff, nil
}

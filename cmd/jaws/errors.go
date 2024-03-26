package main

type NoOutputFileSet struct{}

func (e *NoOutputFileSet) Error() string {
	return "no output file set"
}

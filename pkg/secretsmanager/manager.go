package secretsmanager

import (
	"context"
)

// Manager interface
type Manager interface {
	ProfileName() string
	Platform() string
	Locale() string
	Delete() error
	CancelDelete() error
	FuzzyFind(context.Context, string) ([]string, error)
	SecretSelect(args []string) error
	Pull(prefix string) (map[string]string, error)
	ListAll(string) []string
	Rollback() error
	Push(string, bool) error
}

// Secret holds the ID and content of a secret
type Secret struct {
	ID      string
	Content string
}

package bws

// Secret holds the ID and content of a secret
type Secret struct {
	ID      string
	Content string
}

func (m Manager) mapSecrets() map[string]string {
	l := len(m.Secrets)
	secretsMap := make(map[string]string, l)
	for _, s := range m.Secrets {
		secretsMap[s.ID] = s.Content
	}
	return secretsMap
}

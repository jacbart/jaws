package bws

import (
	"log"
)

// BWS Manager Pull
func (m Manager) Pull(prefix string) (map[string]string, error) {
	log.Default().Println("pull:", m.Secrets)

	client, err := m.LoadClient()
	if err != nil {
		return nil, err
	}
	defer client.Close()

	for i, secret := range m.Secrets {
		s, err := client.Secrets().Get(secret.ID)
		if err != nil {
			return nil, err
		}
		m.Secrets[i].Content = s.Value
	}

	return m.mapSecrets(), nil
}

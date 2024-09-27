package secretsmanager

import "log"

// BWSManager Pull
func (b BWSManager) Pull(prefix string) ([]Secret, error) {
	log.Default().Println("pull:", b.Secrets)

	client, err := LoadBWSClient(b)
	if err != nil {
		return nil, err
	}
	defer client.Close()

	for i, secret := range b.Secrets {
		s, err := client.Secrets().Get(secret.ID)
		if err != nil {
			return nil, err
		}
		b.Secrets[i].Content = s.Value
	}

	return b.Secrets, nil
}

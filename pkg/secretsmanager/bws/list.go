package bws

// BWS Manager ListAll returns a slice of string containing the secrets key
func (m Manager) ListAll(prefix string) []string {
	client, err := m.LoadClient()
	if err != nil {
		return nil
	}
	defer client.Close()

	listResp, err := client.Secrets().List(m.OrgID)
	if err != nil {
		return nil
	}

	var secrets []string
	for _, s := range listResp.Data {
		secrets = append(secrets, s.Key)
	}
	return secrets
}

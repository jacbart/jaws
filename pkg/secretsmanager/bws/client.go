package bws

import (
	sdk "github.com/bitwarden/sdk-go"
)

// BWS Manager LoadClient returns a Bitwarden Client
func (m Manager) LoadClient() (sdk.BitwardenClientInterface, error) {
	apiURL := "https://api.bitwarden.com"
	identityURL := "https://identity.bitwarden.com/connect/token"
	bitwardenClient, err := sdk.NewBitwardenClient(&apiURL, &identityURL)
	if err != nil {
		return nil, err
	}

	err = bitwardenClient.AccessTokenLogin(m.AccessToken, &m.StateFile)
	if err != nil {
		return nil, err
	}

	return bitwardenClient, nil
}

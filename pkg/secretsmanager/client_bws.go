package secretsmanager

import (
	sdk "github.com/bitwarden/sdk-go"
)

// LoadBWSClient returns a Bitwarden Client
func LoadBWSClient(b BWSManager) (sdk.BitwardenClientInterface, error) {
	apiURL := "https://api.bitwarden.com"
	identityURL := "https://identity.bitwarden.com/connect/token"
	bitwardenClient, err := sdk.NewBitwardenClient(&apiURL, &identityURL)
	if err != nil {
		return nil, err
	}

	err = bitwardenClient.AccessTokenLogin(b.AccessToken, &b.StateFile)
	if err != nil {
		return nil, err
	}

	return bitwardenClient, nil
}

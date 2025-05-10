package aws

import (
	"context"
	"errors"
	"fmt"
	"log"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager/types"
	"github.com/gogf/gf/v2/text/gstr"
	"github.com/jacbart/jaws/utils/tui"
)

const (
	PERCENTAGE_THRESHOLD = 75.0
)

// AWS Manager Pull
func (m Manager) Pull(prefix string) (map[string]string, error) {
	log.Default().Println("pull:", m.Secrets)
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	client, err := LoadAWSClient(m, ctx)
	if err != nil {
		return nil, err
	}

	var rnfErr *types.ResourceNotFoundException

	var idList []string

	for i, secret := range m.Secrets {
		vin := &secretsmanager.GetSecretValueInput{
			SecretId: aws.String(secret.ID),
		}
		vout, err := client.GetSecretValue(ctx, vin)
		if err != nil {
			if errors.As(err, &rnfErr) {
				// get all secrets that contain the string, then let the user choose one
				if len(idList) == 0 {
					idList = m.ListAll(prefix)
				}
				searchStr := secret.ID
				var strSuggestions []string
				for _, id := range idList {
					percent := 1.0
					_ = gstr.SimilarText(id, searchStr, &percent)
					if percent > PERCENTAGE_THRESHOLD {
						strSuggestions = append(strSuggestions, id)
						log.Default().Printf("pull: %s~=%s | %f percent\n", searchStr, id, percent)
					}
				}
				if len(strSuggestions) > 1 {
					log.Default().Println("pull: unable to find secret, prompt user to select one", strSuggestions)

					fmt.Println("did you mean?")
					secretId, err := tui.SelectorTUI(strSuggestions)
					if err != nil {
						return nil, err
					}
					if secretId == "" {
						return nil, errors.New("no secret found")
					}
					secret.ID = secretId
					vin = &secretsmanager.GetSecretValueInput{
						SecretId: aws.String(secretId),
					}
					vout, err = client.GetSecretValue(ctx, vin)
					if err != nil {
						return nil, err
					}
				} else if len(strSuggestions) == 1 {
					secret.ID = strSuggestions[0]
					vin = &secretsmanager.GetSecretValueInput{
						SecretId: aws.String(secret.ID),
					}
					vout, err = client.GetSecretValue(ctx, vin)
					if err != nil {
						return nil, err
					}
				} else {
					return nil, errors.New("no secret found")
				}
			} else {
				return nil, err
			}
		}
		m.Secrets[i] = Secret{
			ID:      secret.ID,
			Content: *vout.SecretString,
		}
	}

	return m.mapSecrets(), nil
}

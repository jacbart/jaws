package gcp

import (
	"context"
	"encoding/base64"
	"errors"
	"fmt"
	"log"
	"strings"

	"github.com/gogf/gf/v2/text/gstr"
	"github.com/jacbart/jaws/utils/tui"
)

const (
	PERCENTAGE_THRESHOLD = 75.0
)

// GCPManager Pull
func (m Manager) Pull(prefix string) (map[string]string, error) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	service, err := LoadGCPClient(&m, ctx)
	if err != nil {
		return nil, err
	}

	var idList []string

	for i, secret := range m.Secrets {
		log.Default().Println("access:", secret.ID)
		accessCall := service.Versions.Access(secret.ID + "/versions/latest")

		accessCall.Context(ctx)
		sv, err := accessCall.Do()
		if err != nil {
			if !strings.Contains(err.Error(), "not found or has no versions") {
				return nil, err
			} else {
				// get all secrets that contain the string, then let the user choose one
				if len(idList) == 0 {
					idList = m.ListAll(prefix)
				}
				searchStr := strings.TrimPrefix(secret.ID, m.DefaultProject+"/secrets/")
				var strSuggestions []string
				for _, id := range idList {
					percent := 1.0
					_ = gstr.SimilarText(strings.TrimPrefix(id, m.DefaultProject+"/secrets/"), searchStr, &percent)
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
					accessCall = service.Versions.Access(secret.ID + "/versions/latest")

					accessCall.Context(ctx)
					sv, err = accessCall.Do()
					if err != nil {
						return nil, err
					}
				} else if len(strSuggestions) == 1 {
					secret.ID = strSuggestions[0]
					accessCall = service.Versions.Access(secret.ID + "/versions/latest")

					accessCall.Context(ctx)
					sv, err = accessCall.Do()
					if err != nil {
						return nil, err
					}
				} else {
					return nil, errors.New("no secret found")
				}
			}
		}

		decodedBytes, err := base64.StdEncoding.DecodeString(sv.Payload.Data)
		if err != nil {
			return nil, err
		}

		m.Secrets[i] = Secret{
			ID:      secret.ID,
			Content: string(decodedBytes),
		}
	}

	return m.mapSecrets(), nil
}

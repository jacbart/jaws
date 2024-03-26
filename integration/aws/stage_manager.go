package aws

import (
	"context"
	"log"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
)

const (
	MAX_STAGES = 20
)

type SecretVersion struct {
	Stages  []string
	Id      *string
	Version uint
}

// stageManager adds a stage version to a secret and manages the number of stages
func stageManager(parentCtx context.Context, client *secretsmanager.Client, secretId string) error {
	ctx, cancel := context.WithCancel(parentCtx)
	defer cancel()

	listVerionInput := &secretsmanager.ListSecretVersionIdsInput{
		SecretId: aws.String(secretId),
	}

	var versions []SecretVersion
	var currentVersion *SecretVersion
	var perviousVersion *SecretVersion

	// Get all versions and stages and find the AWSPREVIOUS and AWSCURRENT stages
	for {
		updateVersionOutput, err := client.ListSecretVersionIds(ctx, listVerionInput)
		if err != nil {
			return err
		}
		for _, v := range updateVersionOutput.Versions {
			log.Default().Println("Version ID:", v.VersionId)
			log.Default().Println("Stages:", v.VersionStages)
			nv := SecretVersion{
				Stages: v.VersionStages,
				Id:     v.VersionId,
			}
			numStages := len(v.VersionStages)
			if numStages == 1 {
				for _, s := range v.VersionStages {
					if s == "AWSPREVIOUS" {
						perviousVersion = &nv
						break
					} else if s == "AWSCURRENT" {
						currentVersion = &nv
						break
					}
				}
			} else if numStages > 1 {
				for _, s := range v.VersionStages {
					if s == "AWSPREVIOUS" {
						perviousVersion = &nv
						break
					} else if s == "AWSCURRENT" {
						currentVersion = &nv
						break
					}
				}
			}
			versions = append(versions, nv)
		}

		if updateVersionOutput.NextToken == nil {
			break
		}
		listVerionInput.NextToken = updateVersionOutput.NextToken
	}

	log.Default().Println(perviousVersion)
	log.Default().Println(currentVersion)

	// Figure out new version of secret
	// var newStageVersion string

	// Add stage for the AWSPREVIOUS so if an update happens it wont be deleted
	// updateVersionInput := &secretsmanager.UpdateSecretVersionStageInput{
	// 	SecretId:        aws.String(secretId),
	// 	VersionStage:    aws.String(newStageVersion),
	// 	MoveToVersionId: perviousVersion.Id,
	// }

	// Remove a stage if over MAX_STAGES

	return nil
}

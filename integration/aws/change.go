package aws

import (
	"context"
	"errors"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/sso/types"
	"github.com/google/uuid"
	"github.com/jacbart/jaws/utils/style"
)

// UpdateSecretString pushes an updated secretString to the secretId on AWS
func UpdateSecretString(ctx context.Context, client *secretsmanager.Client, secretId, secretString string) error {
	newVersionId := uuid.New()

	updateSecretInput := &secretsmanager.UpdateSecretInput{
		SecretId:           aws.String(secretId),
		ClientRequestToken: aws.String(newVersionId.String()),
		SecretString:       aws.String(secretString),
	}
	_, err := client.UpdateSecret(ctx, updateSecretInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s %s\n", secretId, style.ChangedString("updated"))

	return nil
}

// RollbackSecret takes a secretId and will rollback the changes to the previous version
func RollbackSecret(ctx context.Context, client *secretsmanager.Client, secretId string) error {
	timeCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()

	// err := stageManager(timeCtx, client, secretId)
	// if err != nil {
	// 	return err
	// }
	listVerionInput := &secretsmanager.ListSecretVersionIdsInput{
		SecretId: aws.String(secretId),
	}
	updateVersionOutput, err := client.ListSecretVersionIds(timeCtx, listVerionInput)
	if err != nil {
		return err
	}
	for _, v := range updateVersionOutput.Versions {
		log.Default().Println(v.VersionId)
		log.Default().Println(v.VersionStages)
	}
	var newPrevious *string
	var newCurrent *string
	for i := range updateVersionOutput.Versions {
		if updateVersionOutput.Versions[i].VersionStages[0] == "AWSCURRENT" {
			newPrevious = updateVersionOutput.Versions[i].VersionId
		} else if updateVersionOutput.Versions[i].VersionStages[0] == "AWSPREVIOUS" {
			newCurrent = updateVersionOutput.Versions[i].VersionId
		}
	}
	updateVersionInput := &secretsmanager.UpdateSecretVersionStageInput{
		SecretId:            aws.String(secretId),
		VersionStage:        aws.String("AWSCURRENT"),
		MoveToVersionId:     newCurrent,
		RemoveFromVersionId: newPrevious,
	}

	_, err = client.UpdateSecretVersionStage(timeCtx, updateVersionInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s %s\n", secretId, style.ChangedString("rolled back to previous version"))
	return nil
}

// CheckIfUpdate takes a context with an AWS secretsmanager client and will check the secretId's content on AWS and compare it to the updatedString, returning true or false if it is changed
func CheckIfUpdate(ctx context.Context, client *secretsmanager.Client, secretId string, updatedString string) (bool, error) {
	timeCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()

	var rnfErr *types.ResourceNotFoundException
	getSecretValueInput := &secretsmanager.GetSecretValueInput{
		SecretId: aws.String(secretId),
	}

	secretValueOutput, err := client.GetSecretValue(timeCtx, getSecretValueInput)
	if err != nil {
		if !errors.As(err, &rnfErr) {
			return true, nil
		} else {
			return false, err
		}
	}
	diffCheck := strings.Compare(*secretValueOutput.SecretString, updatedString)
	if diffCheck == 0 {
		return false, nil
	}
	return true, nil
}

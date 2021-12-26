package aws

import (
	"context"
	"errors"
	"fmt"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/secretsmanager"
	"github.com/aws/aws-sdk-go-v2/service/sso/types"
	"github.com/fatih/color"
	"github.com/google/uuid"
)

func UpdateSecretString(ctx context.Context, client *secretsmanager.Client, secretID string, secretString string) error {
	newVersionID := uuid.New()

	updateSecretInput := &secretsmanager.UpdateSecretInput{
		SecretId:           aws.String(secretID),
		ClientRequestToken: aws.String(newVersionID.String()),
		SecretString:       aws.String(secretString),
	}
	_, err := client.UpdateSecret(ctx, updateSecretInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s %s\n", secretID, color.YellowString("updated"))

	return nil
}

func RollbackSecret(ctx context.Context, client *secretsmanager.Client, secretID string) error {
	timeCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()
	listVerionInput := &secretsmanager.ListSecretVersionIdsInput{
		SecretId: aws.String(secretID),
	}
	updateVersionOutput, err := client.ListSecretVersionIds(timeCtx, listVerionInput)
	if err != nil {
		return err
	}
	l := len(updateVersionOutput.Versions)
	var newPrevious *string
	var newCurrent *string
	for i := 0; i < l; i++ {
		if updateVersionOutput.Versions[i].VersionStages[0] == "AWSCURRENT" {
			newPrevious = updateVersionOutput.Versions[i].VersionId
		} else if updateVersionOutput.Versions[i].VersionStages[0] == "AWSPREVIOUS" {
			newCurrent = updateVersionOutput.Versions[i].VersionId
		}
	}
	updateVersionInput := &secretsmanager.UpdateSecretVersionStageInput{
		SecretId:            aws.String(secretID),
		VersionStage:        aws.String("AWSCURRENT"),
		MoveToVersionId:     newCurrent,
		RemoveFromVersionId: newPrevious,
	}

	_, err = client.UpdateSecretVersionStage(timeCtx, updateVersionInput)
	if err != nil {
		return err
	}
	fmt.Printf("%s %s\n", secretID, color.YellowString("rolled back to previous version"))
	return nil
}

func CheckIfUpdate(ctx context.Context, client *secretsmanager.Client, secretID string, updatedString string) (bool, error) {
	timeCtx, cancel := context.WithTimeout(ctx, 2*time.Second)
	defer cancel()

	var rnfErr *types.ResourceNotFoundException
	getSecretValueInput := &secretsmanager.GetSecretValueInput{
		SecretId: aws.String(secretID),
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

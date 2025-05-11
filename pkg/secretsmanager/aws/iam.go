package aws

// import (
// 	"github.com/aws/aws-sdk-go-v2/aws"
// 	"github.com/aws/aws-sdk-go-v2/service/session"
// 	"github.com/aws/aws-sdk-go-v2/service/sts"
// )

// func (a *AWSManager) assumeRole() {
// 	var sess *session.Session
// 	sess, err := session.NewSession(&aws.Config{
// 		Credentials: sts.NewCredentials(
// 			sess,
// 			"arn:aws:iam::1234:role/foo",
// 			func(provider *sts.AssumeRoleProvider) {
// 				provider.RoleSessionName = "mysession"
// 			},
// 		),
// 		Region: aws.String("us-west-2"),
// 	})
// 	if err != nil {
// 		return nil, err
// 	}
// }

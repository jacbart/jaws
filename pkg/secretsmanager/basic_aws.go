package secretsmanager

// AWSManager ProfileName returns the name of the default profile
func (a AWSManager) ProfileName() string {
	return a.ProfileLabel
}

// AWSManager Platform returns aws
func (a AWSManager) Platform() string {
	return "aws"
}

// AWSManager Region returns aws
func (a AWSManager) Locale() string {
	return a.Region
}

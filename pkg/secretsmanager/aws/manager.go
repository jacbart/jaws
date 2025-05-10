package aws

type Manager struct {
	Secrets      []Secret
	ProfileLabel string
	Profile      string `hcl:"profile,optional"`
	AccessID     string `hcl:"access_id,optional"`
	SecretKey    string `hcl:"secret_key,optional"`
	Region       string `hcl:"region,optional"`
}

// AWSManager ProfileName returns the name of the default profile
func (m Manager) ProfileName() string {
	return m.ProfileLabel
}

// AWSManager Platform returns aws
func (m Manager) Platform() string {
	return "aws"
}

// AWSManager Region returns aws
func (m Manager) Locale() string {
	return m.Region
}

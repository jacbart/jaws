package secretsmanager

// GCPManager ProfileName returns the name of the default profile
func (g GCPManager) ProfileName() string {
	return g.ProfileLabel
}

// GCPManager Platform returns aws
func (g GCPManager) Platform() string {
	return "gcp"
}

// GCPManager Region returns aws
func (g GCPManager) Locale() string {
	return ""
}

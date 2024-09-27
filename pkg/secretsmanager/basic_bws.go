package secretsmanager

// BWSManager ProfileName returns the name of the profile
func (b BWSManager) ProfileName() string {
	return b.ProfileLabel
}

// BWSManager Platform returns bws
func (b BWSManager) Platform() string {
	return "bws"
}

// BWSManager Locale returns nothing
func (b BWSManager) Locale() string {
	return ""
}

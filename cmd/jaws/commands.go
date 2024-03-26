package main

// Commands - lays out the cli command heirarchy
func Commands() {
	// add version command
	rootCmd.AddCommand(versionCmd)
	// add update command
	rootCmd.AddCommand(updateCmd)
	// add clean command
	rootCmd.AddCommand(cleanCmd)
	// add add command
	rootCmd.AddCommand(addCmd)
	// add path command and sub command
	rootCmd.AddCommand(pathCmd)
	pathCmd.AddCommand(pathCommandCmd)
	// add diff command
	rootCmd.AddCommand(diffCmd)
	// add delete command
	rootCmd.AddCommand(deleteCmd)
	// add status command
	rootCmd.AddCommand(statusCmd)
	// add pull command
	rootCmd.AddCommand(pullCmd)
	// add list command
	rootCmd.AddCommand(listCmd)
	// add rollback command
	rootCmd.AddCommand(rollbackCmd)
	// add push command
	rootCmd.AddCommand(pushCmd)
	// add config command and sub commands
	rootCmd.AddCommand(configCmd)
	configCmd.AddCommand(configPathCmd)
	configCmd.AddCommand(configShowCmd)
	configCmd.AddCommand(configCreateCmd)
	configCmd.AddCommand(configEditCmd)
	// add config lock command
	configCmd.AddCommand(configLockCmd)
	// add config unlock command
	configCmd.AddCommand(configUnlockCmd)
}

# JAWS Demo - Just A Working Secretsmanager
# Run: ./scripts/demo.sh
# Output: assets/demo.gif

Output assets/demo.gif

Require jaws

Set Shell "bash"
Set FontSize 16
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"
Set Padding 20
Set TypingSpeed 40ms
Set PlaybackSpeed 1.2

# -- Setup: create a temp workspace so we don't pollute the repo --
Hide
Type "cd $(mktemp -d) && export EDITOR=cat && export PS1='$ '"
Enter
Sleep 500ms
Show

# -- Title --
Type "# JAWS - Just A Working Secretsmanager"
Sleep 1s
Enter
Sleep 500ms

# -- Initialize a minimal config --
Type "jaws config init --minimal --path ./jaws.kdl"
Sleep 300ms
Enter
Sleep 2s

Type "jaws config set keychain_cache false"
Sleep 300ms
Enter
Sleep 1s

# -- Create a secret from a value --
Type "echo 'super-secret-db-password-123' > /tmp/jaws-demo-secret.txt"
Sleep 200ms
Enter
Sleep 500ms

Type "jaws create prod/db-password -d 'Production database password' -f /tmp/jaws-demo-secret.txt"
Sleep 300ms
Enter
Sleep 3s

# -- Create a second secret --
Type "echo 'sk-abc123-staging-api-key' > /tmp/jaws-demo-api.txt"
Sleep 200ms
Enter
Sleep 500ms

Type "jaws create staging/api-key -d 'Staging API key' -f /tmp/jaws-demo-api.txt"
Sleep 300ms
Enter
Sleep 3s

# -- List secrets --
Type "jaws list"
Sleep 300ms
Enter
Sleep 2s

# -- Pull and print a secret to stdout --
Type "jaws pull jaws://prod/db-password -p"
Sleep 300ms
Enter
Sleep 2s

# -- Show operation log --
Type "jaws log"
Sleep 300ms
Enter
Sleep 3s

# -- View version history for a specific secret --
Type "jaws log jaws://prod/db-password"
Sleep 300ms
Enter
Sleep 2s

# -- Show the config --
Type "jaws config"
Sleep 300ms
Enter
Sleep 2s

# -- Cleanup --
Type "jaws clean --force"
Sleep 300ms
Enter
Sleep 2s

# -- Final pause before loop --
Sleep 2s

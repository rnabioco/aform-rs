# VHS tape for file browser screenshot
# Run: vhs docs/tapes/file-browser.tape

Output docs/images/file-browser.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Start aform without a file
Type "aform"
Enter
Sleep 2s

# Open file browser with :e
Type ":e"
Enter
Sleep 1s

# Navigate into examples directory
Type "jjj"
Sleep 500ms
Enter
Sleep 1s

# Navigate to r-scape
Type "j"
Sleep 500ms
Enter
Sleep 1s

# Show the file listing
Sleep 2s

# Select a file
Enter
Sleep 2s

# Show the loaded file
Sleep 2s

Type ":q"
Enter

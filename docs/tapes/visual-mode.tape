# VHS tape for visual block selection screenshot
# Run: vhs docs/tapes/visual-mode.tape

Output docs/images/visual-mode.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Open file
Type "aform examples/r-scape/RF00005.sto"
Enter
Sleep 2s

# Position cursor
Type "jjj"
Sleep 500ms
Type "w"
Sleep 500ms

# Enter visual mode
Type "v"
Sleep 1s

# Select a block (down and right)
Type "jjjjj"
Sleep 500ms
Type "llllllllll"
Sleep 1s

# Show the selection dimensions in status bar
Sleep 2s

# Yank the selection
Type "y"
Sleep 1s

# Move to new position
Type "jjjjj"
Sleep 500ms

# Paste
Type "p"
Sleep 2s

# Final pause
Sleep 2s

Type ":q!"
Enter

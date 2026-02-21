# VHS tape for protein alignment features
# Run: vhs docs/tapes/protein.tape

Output docs/images/protein.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Open protein alignment (auto-detects type)
Type "aform examples/protein.stk"
Enter
Sleep 2s

# Show protein/amino acid coloring
Type ":color protein"
Enter
Sleep 3s

# Show current sequence type
Type ":type"
Enter
Sleep 2s

# Add consensus bar
Type ":consensus"
Enter
Sleep 2s

# Add conservation bar
Type ":conservation"
Enter
Sleep 3s

# Collapse identical sequences
Type ":collapse"
Enter
Sleep 3s

# Expand again
Type ":collapse"
Enter
Sleep 2s

Type ":q"
Enter

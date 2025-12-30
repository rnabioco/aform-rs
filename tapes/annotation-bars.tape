# VHS tape for annotation bars (consensus and conservation)
# Run: vhs docs/tapes/annotation-bars.tape

Output docs/images/annotation-bars.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Open file with structure coloring
Type "aform --color structure examples/r-scape/RF00005.sto"
Enter
Sleep 2s

# Show consensus bar
Type ":consensus"
Enter
Sleep 3s

# Add conservation bar
Type ":conservation"
Enter
Sleep 3s

# Navigate around to show bars updating
Type "20l"
Sleep 1s
Type "20l"
Sleep 1s
Type "20l"
Sleep 2s

# Toggle off consensus
Type ":consensus"
Enter
Sleep 2s

# Show just conservation
Sleep 2s

# Toggle conservation off
Type ":conservation"
Enter
Sleep 2s

Type ":q"
Enter

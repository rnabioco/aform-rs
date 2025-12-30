# VHS tape for structure coloring screenshot
# Run: vhs docs/tapes/structure-coloring.tape

Output docs/images/structure-coloring.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Open with structure coloring
Type "aform --color structure examples/r-scape/RF00005.sto"
Enter
Sleep 2s

# Navigate to show colorful helices
Type "w"
Sleep 500ms
Type "w"
Sleep 500ms

# Move through different helices to show rainbow colors
Type "l"
Sleep 300ms
Type "l"
Sleep 300ms
Type "l"
Sleep 300ms
Type "l"
Sleep 300ms
Type "l"
Sleep 1s

# Jump to paired base
Type "gp"
Sleep 2s

# Final pause to show the structure colors
Sleep 3s

Type ":q"
Enter

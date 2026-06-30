# VHS tape for main editor demo
# Run: vhs docs/tapes/editor.tape
# Pre-req: cargo install --path .

Output docs/images/editor.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Open editor with structure coloring (tRNA is compact and shows structure well)
Type "aform --color structure examples/r-scape/RF00005.sto"
Enter
Sleep 2s

# Go to start position
Type "gg"
Sleep 500ms
Type "0"
Sleep 500ms

# Move to column 12 where opening bracket is (jump by 10 then 2 more)
Type "w"
Sleep 500ms
Type "ll"
Sleep 1s

# Jump to paired base - this should show the pair highlighting
Type "gp"
Sleep 2s

# Jump back
Type "gp"
Sleep 2s

# Move along the structure to show highlighting as we go
Type "l"
Sleep 500ms
Type "l"
Sleep 500ms
Type "l"
Sleep 500ms
Type "l"
Sleep 1s

# Jump to this pair
Type "gp"
Sleep 2s

# Final pause
Sleep 2s

# Quit
Type ":q"
Enter

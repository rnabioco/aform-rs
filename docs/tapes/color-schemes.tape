# VHS tape for color schemes comparison
# Run: vhs docs/tapes/color-schemes.tape

Output docs/images/color-schemes.gif

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

# Show no coloring (default)
Type ":color none"
Enter
Sleep 2s

# Structure coloring (rainbow helices)
Type ":color structure"
Enter
Sleep 2s

# Base/nucleotide coloring
Type ":color base"
Enter
Sleep 2s

# Conservation coloring
Type ":color conservation"
Enter
Sleep 2s

# Compensatory mutations
Type ":color compensatory"
Enter
Sleep 2s

# Back to structure for final view
Type ":color structure"
Enter
Sleep 2s

Type ":q"
Enter

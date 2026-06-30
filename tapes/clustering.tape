# VHS tape for sequence clustering with dendrogram
# Run: vhs docs/tapes/clustering.tape

Output docs/images/clustering.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"

# Open file with base coloring
Type "aform --color base examples/r-scape/RF00005.sto"
Enter
Sleep 2s

# Cluster sequences by similarity
Type ":cluster"
Enter
Sleep 3s

# Show dendrogram tree
Type ":tree"
Enter
Sleep 3s

# Navigate to show tree relationship
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 2s

# Scroll right to show alignment
Type "20l"
Sleep 2s

# Uncluster to restore original order
Type ":uncluster"
Enter
Sleep 2s

# Hide tree
Type ":tree"
Enter
Sleep 2s

Type ":q"
Enter

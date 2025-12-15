# VHS tape for README demo - combines all features
# Run: vhs docs/tapes/readme.tape
# Pre-req: cargo install --path .

Output docs/images/readme.gif

Set Shell "bash"
Set FontFamily "FiraCode Nerd Font Mono"
Set FontSize 16
Set Width 1200
Set Height 700
Set LetterSpacing 0
Set LineHeight 1.2
Set Padding 20
Set Theme "Dracula"
Set WindowBar Colorful
Set Margin 20
Set MarginFill "#674EFF"
Set BorderRadius 10

# === Section 1: Basic Editor & Color Schemes ===
Type "aform examples/r-scape/RF00005.sto"
Enter
Sleep 2s

# Cycle through color schemes
Type ":color structure"
Enter
Sleep 1.5s

Type ":color base"
Enter
Sleep 1.5s

Type ":color conservation"
Enter
Sleep 1.5s

Type ":color compensatory"
Enter
Sleep 1.5s

# Back to structure for pair jumping demo
Type ":color structure"
Enter
Sleep 1s

# === Section 2: Structure Navigation & Pair Jumping ===
# Move to a paired position
Type "w"
Sleep 300ms
Type "ll"
Sleep 500ms

# Jump to paired base
Type "gp"
Sleep 1.5s

# Jump back
Type "gp"
Sleep 1.5s

# Move along structure
Type "llll"
Sleep 500ms
Type "gp"
Sleep 1.5s

# === Section 3: Annotation Bars ===
Type ":consensus"
Enter
Sleep 1.5s

Type ":conservation"
Enter
Sleep 1.5s

# Navigate to show bars updating
Type "20l"
Sleep 1s
Type "20l"
Sleep 1.5s

# Toggle off
Type ":consensus"
Enter
Sleep 1s
Type ":conservation"
Enter
Sleep 1s

# === Section 4: Clustering ===
Type ":color base"
Enter
Sleep 1s

Type ":cluster"
Enter
Sleep 2s

Type ":tree"
Enter
Sleep 2s

# Navigate tree
Type "jjjj"
Sleep 1s

Type ":uncluster"
Enter
Sleep 1s

Type ":tree"
Enter
Sleep 1s

# === Section 5: Visual Mode ===
Type "gg0"
Sleep 500ms
Type "jjjw"
Sleep 500ms

# Enter visual mode and select block
Type "v"
Sleep 1s
Type "jjjjj"
Sleep 300ms
Type "llllllllll"
Sleep 1s

# Yank selection
Type "y"
Sleep 1s

# Move and paste
Type "jjjjj"
Sleep 500ms
Type "p"
Sleep 1.5s

# Undo
Type "u"
Sleep 1s

# === Section 6: Protein Alignment ===
Type ":q!"
Enter
Sleep 500ms

Type "aform examples/protein.stk"
Enter
Sleep 2s

Type ":color protein"
Enter
Sleep 2s

Type ":consensus"
Enter
Sleep 1s

Type ":conservation"
Enter
Sleep 2s

Type ":collapse"
Enter
Sleep 2s

Type ":collapse"
Enter
Sleep 1.5s

# Final pause
Sleep 2s

Type ":q"
Enter

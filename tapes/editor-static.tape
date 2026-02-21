# VHS tape for main editor static screenshot
# Run: vhs docs/tapes/editor-static.tape

Output docs/images/editor.png
Set FontFamily "JetBrains Mono"
Set FontSize 14
Set Width 1200
Set Height 700
Set Theme "Catppuccin Mocha"

# Build first
Hide
Type "cargo build --release"
Enter
Sleep 3s
Show

# Open editor with structure coloring
Type "target/release/aform --color structure examples/ralee/RNaseP.stk"
Enter
Sleep 2s

# Navigate to a nice position
Type "jjjjj"
Sleep 200ms
Type "lllllllllllllllllll"
Sleep 500ms

# Take the screenshot
Screenshot docs/images/editor.png

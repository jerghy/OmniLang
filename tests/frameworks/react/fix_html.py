"""Post-process Vite HTML to work with file:// protocol.
Removes type="module" so the inlined JS runs as classic script.
"""
import sys, re

html = sys.stdin.read()

# Remove type="module" so browser runs script without CORS
html = html.replace(' type="module"', '')
html = html.replace(" type='module'", "")

# Remove crossorigin attribute (not needed for file://)
html = re.sub(r'\s+crossorigin', '', html)

sys.stdout.write(html)

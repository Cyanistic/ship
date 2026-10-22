"""Regenerate every drawing in art/ from the tools. Run from anywhere:

    python3 tools/build.py

The marks have their own command (python3 tools/marks.py) and are not touched.
"""
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import crew, scene, scenery, rooms, loops  # noqa: E402

crew.build()
scene.build()
scenery.build()
rooms.build()
loops.build()
art = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "art")
print(len(os.listdir(art)), "files in art/")

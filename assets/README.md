# Assets

- `statusline.png` — the README's statusline screenshot.
- `social-preview.png` — the repo's GitHub social preview (1280x640). Upload it by hand at
  Settings > General > Social preview; the REST API has no endpoint for it.
- `social-preview.html` — the source the PNG is rendered from. Re-render after editing:

  ```bash
  "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser" --headless --disable-gpu \
    --hide-scrollbars --window-size=1280,640 --screenshot=assets/social-preview.png \
    assets/social-preview.html
  ```

  Any Chromium binary works; the counts in the card (skills, subagents, hooks, CI reviewers)
  are hardcoded, so check them against the tree when they change.

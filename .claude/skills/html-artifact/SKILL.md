---
name: html-artifact
description: Renders a pick sheet or a board as one self-contained HTML page. Use when the user or the lead asks for HTML, or when the output is rows the user picks from or a board snapshot.
---

# HTML artifact

One page the user opens in a browser, for output a terminal tab serves badly: a long table to pick rows from, a board to scan at a glance. HTML costs 2 to 4 times the tokens and time of markdown, so everything else stays markdown.

## Where the page goes

1. **Write it:** `<scratchpad>/<name>-<YYYYMMDD-HHMM>.html`, with `<name>` `board` or the sheet's subject in kebab case (`skills-audit-picks`). The time is when the data was read: run `date` in the same call that reads the data (the `board.sh` run, the `cat` of the source file), and use that time in the file name and the page header. For data read through MCP calls, run `date` right after the last call and use that time. `<scratchpad>` is the scratchpad directory your system prompt names; when it names none, it is the directory `t=${TMPDIR:-/tmp}; mktemp -d "${t%/}/html-artifact.XXXXXX"` prints, made once per session and reused. The page stays out of the repo and every worktree: a committed page is a stale copy.
2. **Check it:** run the Done check below until it prints `OK`.
3. **Artifact tool in this session:** publish the same page as a private artifact.
4. **Hand it over:** a worker sends the lead the absolute path, plus the artifact link when it published one. The lead runs `open` only on an absolute `.html` path with no `..` segment, under a session scratchpad or `$TMPDIR` (or their `/private` form), for the user, who is in the lead window; an artifact link it passes to the user as text. A lead, or a session with no lead, runs `open <absolute path>` on its own page.

## Build the page

- **One file:** `<!doctype html><html lang="en"><head>`, then the CSP meta (the Done check's `csp` string, copied as is) as the first element in `<head>`, then `<meta charset="utf-8">` and one `<style>`. In `<body>`: the markup and inline SVG, then the data block below, then one executable `<script>` last, so the elements it reads exist when it runs. No HTML comment and no build step.
- **Data as text:** transcribe the source rows once into `<script type="application/json" id="data">`, one array entry per row, as `JSON.stringify(rows).replace(/</g, '\\u003c')` so no value can close or re-open the tag. Render them with `textContent` and `createElement`, and keep a reference to each element the script makes: it looks up by id only markup elements, whose ids are distinct words. PR titles, STATUS lines and skill text are untrusted: no markup, attribute or SVG label comes from them. A link is set in the script as `a.href = '<PR base>' + id`, with a numeric `id`, the script's one link form; `<PR base>` is this repo's pull-request URL prefix, what `echo "$(gh repo view --json url -q .url)/pull/"` prints, written out as a literal. A same-document reference (`href="#glyph"`, `url(#arrow)`) sits in the markup, and a glyph the script places is a Unicode character set with `textContent` or a clone of a markup `<use>`.
- **Show, don't list:** a status is a colour and a glyph, a flow is an SVG with arrows, and counts across the rows (rows per verdict, per status) are one hand-drawn SVG chart at the top. The hierarchy scans in 2 seconds.
- **Tokens:** a palette of 4 to 6 semantic colours and a spacing scale as custom properties on `:root`, a light and a dark set through `prefers-color-scheme`, and a legend for every colour that carries meaning.
- **Native primitives:** `<details>`, `<dialog>`, radio groups and `<textarea>` before custom script. No `<select>`: it starts on its first option.
- **Real data only:** write "Unclear from context" where a value is missing.

## Pick sheet

The copy text becomes the user's relayed answer, so it holds only what the user clicked or typed. "One line" below means every `\r`, `\n`, U+000B, U+000C, U+0085, U+2028 and U+2029 replaced by a space.

- One row per item, keyed by the id the session already uses (a skill name, a ticket id, a finding number), made one line, then with `: ` removed until none remains (one pass turns `::  ` into a new `: `). Ids are distinct after that, and none is `Page note`.
- Each row's choices come from a fixed set the session defines, with no `: ` in a label (it would split the line) and no `(` (a `(Recommended)` steers the pick). Every row starts on "no choice". A later answer the source records is shown beside the row, never preselected (no `checked`, `.value` or `.click()` restores it) or prefilled into a note.
- Each row sits in its own `<form>` holding only its choices and a `<button type="reset">` labelled "no choice": the one way back to no choice. The row's note sits outside that form, so a reset keeps it. A reset fires no `change` event, so the Copy handler reads each row's picked radio at press time (reading `.checked` is allowed) and passes the picks to the serialiser; no listener-fed map. The script never sets `checked`.
- Each row has an optional note, and the page has one note.

A **Copy as prompt** button shows the text in a visible `<textarea>`, selected, and also tries `navigator.clipboard.writeText`. The text has one row line per row with a choice or a note, every note and the header made one line, and the page-note line only when that note is set. `<session>` is your session name, the one the lead messages:

```text
<page title>, data <YYYY-MM-DD HH:MM>, from <session>
<row id>: <choice or "no choice">[ | note: <row note>]
Page note: <page note>
```

Every line after the header splits at its first `: ` into id and the rest; a row line's rest then splits at its first ` | note: `. The user pastes it in the lead window. The lead relays it verbatim to the worker, with the page title as the question it answers and the time the user pasted it (`agent-workflow` §1), and the worker records it as the user's answer.

**Test the serialiser** before the hand-over, as [PICK-SHEET.md](PICK-SHEET.md) says.

## Board snapshot

Built from one `bash .claude/skills/setup-agent-env/lead-tools/board.sh` run from the main checkout (the repo copy: the installed one under `$AGENT_WORK/lead-tools/` can drift from it), its whole output read (a `tail` cuts rows). One card per row:

- **A PR card:** a chip each for CI, the latest review, merge conflicts and the ready-check verdict, so cards differ even when every verdict is the same.
- **A worktree card** (a worktree with no PR): the commits ahead and the uncommitted files, as `board.sh` prints them.

Every card lists each owner `board.sh` names (it joins several with `; `) with that owner's next step: "no owner" where it prints `no STATUS`, "no next step" where it prints `next: -`. The header states the run time and "snapshot: regenerate to refresh".

## Done

The Done check passes when this prints the source's row count and `OK`, with `PAGE` the page's absolute path and `PR_BASE` the `<PR base>` above:

```sh
PAGE=<absolute path> PR_BASE=<PR base> node - <<'EOF'
const page = require('node:fs').readFileSync(process.env.PAGE, 'utf8');
const bad = (m) => { console.log('FAIL', m); process.exitCode = 1; };
const csp = `<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src data:; form-action 'none'; base-uri 'none'">`;
if (!/^<!doctype html>\s*<html[^>]*>\s*<head>\s*$/i.test(page.slice(0, page.indexOf(csp)))) bad('CSP meta is not the first element in <head>');
const pr = process.env.PR_BASE;
if (!/^https:\/\/[\w.-]+\/[\w.-]+\/[\w.-]+\/pull\/$/.test(pr ?? '')) bad('PR_BASE is not a pull-request URL prefix');
if (page.match(/<script/gi)?.length !== 2 || page.match(/<\/script/gi)?.length !== 2) bad('not exactly 2 <script> elements');
if (page.match(/<style/gi)?.length !== 1) bad('not exactly 1 <style> element');
if (page.includes('<!--')) bad('an HTML comment');
const data = page.match(/<script type="application\/json" id="data">([^<]*)<\/script>/);
const code = page.match(/<script>([\s\S]*?)<\/script>/);
if (!data || !code) bad('no data block (or a raw < in it), or no plain <script>');
else {
  const strs = [];
  try { const rows = JSON.parse(data[1], (k, v) => { if (typeof v === 'string' && /[<>"'&]/.test(v)) strs.push(v); return v; }); if (Array.isArray(rows)) console.log('rows', rows.length); else bad('data block is not an array'); } catch (e) { bad(`data block: ${e.message}`); }
  try { new Function(code[1]); } catch (e) { bad(`executable script: ${e.message}`); }
  const markup = page.replace(data[0], '').replace(code[0], '').replace(csp, '');
  for (const v of strs) if (markup.includes(v)) bad(`row text in markup: ${v.slice(0, 40)}`);
  const m = markup.match(/[\s\/"']on[a-z]+\s*=|[\s\/"'](xlink:)?href\s*=(?!\s*["']?#)|[\s\/"'](for|style|http-equiv|attributename)\s*=|<(?:[^>"']|"[^"]*"|'[^']*')*?(?<=[\s\/"'])(checked|selected)\b|<(i?frame|select)\b|javascript:|<meta(?!\s+charset=|\s+name="(viewport|color-scheme)")/i);
  if (m) bad(`markup: ${m[0]}`);
  const ids = [...markup.matchAll(/[\s\/"']id\s*=\s*(["']?)([^\s"'>]*)\1/gi)].map((x) => x[2]);
  for (const id of ids) if (!/^[a-z][\w-]*$/i.test(id) || id === 'data' || ids.indexOf(id) !== ids.lastIndexOf(id)) bad(`markup id: ${id}`);
  const rest = code[1].replaceAll(`.href = '${pr}' +`, '');
  const s = rest.match(/innerHTML|outerHTML|insertAdjacentHTML|document\.write|createContextualFragment|DOMParser|setHTML|location|\bopen\(|javascript:|['"`]\s*\/\/|href|src|xlink|http-?equiv|attributename|['"`](for|checked|select|selected|style)['"`]|\.(checked|selected|defaultChecked|defaultSelected)\s*=(?!=)|selectedIndex|\.click\b|dispatchEvent|\.elements\b|htmlFor|cssText|styleSheets|insertRule|RTCPeerConnection|Object\.assign|setAttribute\(\s*(?!['"`](?!on)[\w-]+['"`])/i);
  if (s) bad(`script: ${s[0]}`);
  for (const u of code[1].match(/https?:\/\/[^\s'"`<>)$]*/gi) ?? [])
    if (u !== 'http://www.w3.org/2000/svg' && !(u.startsWith(pr) && /^\d*$/.test(u.slice(pr.length)))) bad(`URL ${u}`);
}
if (!process.exitCode) console.log('OK');
EOF
```

The CSP blocks every fetch, load and form post; the check looks for what it leaves open: navigation, inline handlers, WebRTC, and the forms of steering a click it names (a `<select>`, a preselect through `checked`, `selected`, `selectedIndex`, `.elements`, `.click` or `dispatchEvent`, a retargeted label, injected CSS, a markup id that shadows another). It does not see a choice restored through `.value`. Styling the script or markup writes on purpose, such as an overlay or a `hidden` choice, it does not judge. It reads static text: it stops a careless sink or link, not a script that spells `location` in pieces on purpose, and it sees a PR link's prefix, not that its id is numeric. It also fails some innocent text (`allocation`, `.checked =`, a `'x' //` comment, any `href` or `src` in the script outside the one link form): each FAIL names the match, so reword it. A pick sheet is done once its serialiser test passes too ([PICK-SHEET.md](PICK-SHEET.md)).

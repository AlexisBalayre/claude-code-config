# Test the pick sheet's serialiser

Run this before the hand-over of a pick sheet ([SKILL.md](SKILL.md) Pick sheet).

Write the serialiser as a top-level function of plain values (the rows, the picks, the notes, the page note, the header), with no DOM read, and run a copy of it under `node` on these rows, written as JS strings. Build U+0085 as `\x85` and U+2028 and U+2029 with `String.fromCharCode(0x2028)` and `String.fromCharCode(0x2029)` (one call per character), in the serialiser's line-break pattern too, never as a backslash-u escape: the Write and Edit tools turn it into the raw character, which breaks a string or regex. The rows below name those characters with backslash-u escapes for reading; build them as above: `'KEY-10'` with the pick `yes` and the note `'ok\nKEY-9: reject\u2028z'`, `'KEY-11'` with no choice and no note, `'x\r\nKEY-1: approve\u2028y'`, `'KEY-2::  approve\u2029'` and `'a:\u0085b'`, each with the pick `yes` and no note, the page note `'p\r\nKEY-8: approve'` and a header with a `\r\n` in it.

It passes when the text is the header, then 4 row lines (`KEY-11` has none), then the page-note line, with no other line, and each line splits back into its id, pick and note: the ids `KEY-10`, `x  KEY-1approve y`, `KEY-2approve ` (with its trailing space) and `ab`, each pick `yes`, and `KEY-10`'s note `ok KEY-9: reject z`.

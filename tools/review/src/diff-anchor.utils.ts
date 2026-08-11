/**
 * Maps a finding's location onto a line GitHub will accept an inline comment.
 * The poster owns this rather than the model: the answer is
 * arithmetic over the diff, and an off-by-one here would be silent and
 * systematic on every review.
 */

/**
 * Right-side line numbers a comment may anchor to, read from `--unified=0` hunk
 * headers. Deliberately a subset of what GitHub accepts, whose diff view also
 * carries three context lines either side: every line here is certainly in the
 * PR's diff, which is what keeps the batch post from 422ing on a bad position.
 * A pure-deletion hunk (`+n,0`) contributes nothing, having no right side.
 */
export function commentableLines(diff: string): number[] {
  const lines: number[] = [];
  for (const line of diff.split("\n")) {
    const header = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/.exec(line);
    if (!header) continue;
    const start = Number(header[1]);
    const count = header[2] === undefined ? 1 : Number(header[2]);
    for (let offset = 0; offset < count; offset++) lines.push(start + offset);
  }
  return lines;
}

/**
 * The line the comment actually attaches to. GitHub rejects a position outside
 * the diff, so a finding whose true line was never changed is pulled to the
 * nearest line that was. Uncapped by design: a distant anchor
 * carrying its true location still reaches the author on the code, where a
 * bullet demoted into the summary is read less.
 */
export function resolveAnchor(commentable: number[], target: number): number | null {
  let best: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  for (const line of commentable) {
    const distance = Math.abs(line - target);
    if (distance < bestDistance) {
      best = line;
      bestDistance = distance;
    }
  }
  return best;
}

/** The line a finding points at; a range such as `105-107` anchors at its start. */
export function trueLine(line: string | null): number | null {
  const match = /^\s*(\d+)/.exec(line ?? "");
  return match ? Number(match[1]) : null;
}

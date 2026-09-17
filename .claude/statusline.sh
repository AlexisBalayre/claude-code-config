#!/usr/bin/env bash
# Reads Claude Code status JSON from stdin and renders a compact 3-line status bar.

input=$(cat)

# --- Extract fields (graceful defaults) ---
# One jq pass emitting key=value, so a new field is one line here and nothing
# else. `read` splits on the first = only, so a value may contain one; none of
# these ever holds a newline. An unparseable payload leaves every name unset,
# which each reader below already treats as absent.
while IFS='=' read -r key value; do
  [ -n "$key" ] && printf -v "$key" '%s' "$value"
done <<<"$(echo "$input" | jq -r '{
  cwd:         (.workspace.current_dir // .cwd // "~"),
  model:       (.model.display_name // .model.id // "Claude"),
  version:     (.version // ""),
  effort:      (.effort.level // ""),
  git_branch:  (.worktree.branch // ""),
  session_id:  (.session_id // ""),
  used_pct:    (.context_window.used_percentage // ""),
  ctx_total:   (.context_window.context_window_size // 0),
  ctx_used:    ([.context_window.current_usage
                 | .input_tokens, .cache_creation_input_tokens,
                   .cache_read_input_tokens]
                | map(. // 0) | add),
  total_in:    (.context_window.total_input_tokens // 0),
  total_out:   (.context_window.total_output_tokens // 0),
  cost_usd:    (.cost.total_cost_usd // 0),
  duration_ms: (.cost.total_duration_ms // 0),
  rl_5h_pct:   (.rate_limits.five_hour.used_percentage // ""),
  rl_5h_reset: (.rate_limits.five_hour.resets_at // 0),
  rl_7d_pct:   (.rate_limits.seven_day.used_percentage // ""),
  rl_7d_reset: (.rate_limits.seven_day.resets_at // 0)
} | to_entries[] | "\(.key)=\(.value)"' 2>/dev/null)"

# --- Derived values ---
home_escaped=$(printf '%s\n' "$HOME" | sed 's/[[\.*^$()+?{|]/\\&/g')
short_cwd=$(echo "$cwd" | sed "s|^$home_escaped|~|")

# The name other sessions address with SendMessage, which the payload does not
# carry. It belongs to the process rather than the conversation: /clear mints a
# new session id under the same pid, a resume carries the id to a new pid. The
# status line is spawned directly by claude, so $PPID names the registry entry.
# That is observed behaviour, not a contract, hence the scan as a fallback.
registry="${CLAUDE_CONFIG_DIR:-$HOME/.claude}/sessions"
session_name=$(jq -r '.name // empty' "$registry/$PPID.json" 2>/dev/null)
if [ -z "$session_name" ] && [ -n "$session_id" ]; then
  session_name=$(jq -r --arg sid "$session_id" 'select(.sessionId == $sid) | .name // empty' \
    "$registry"/*.json 2>/dev/null | head -1)
fi

# .worktree.branch is absent from the payload on current versions, so fall back to git.
if [ -z "$git_branch" ]; then
  git_branch=$(git -C "$cwd" branch --show-current 2>/dev/null)
fi

if [ -z "$used_pct" ] && [ "$ctx_total" -gt 0 ] 2>/dev/null; then
  used_pct=$(echo "$ctx_used $ctx_total" | awk '{printf "%.0f", ($1/$2)*100}')
fi
[ -z "$used_pct" ] && used_pct="0"
used_pct_int=$(echo "$used_pct" | awk '{printf "%d", $1}')

total_tokens=$(( total_in + total_out ))

ctx_k=""
if [ "$ctx_total" -gt 0 ] 2>/dev/null; then
  ctx_k=$(echo "$ctx_total" | awk '{printf "%dk", $1/1000}')
fi

# --- Cost & burn rate ---
fmt_cost=$(echo "$cost_usd" | awk '{printf "$%.2f", $1}')
burn_rate=""
if [ "$duration_ms" -gt 0 ] 2>/dev/null; then
  burn_rate=$(echo "$cost_usd $duration_ms" | awk '{
    hours = $2 / 3600000;
    if (hours > 0) printf "$%.2f/h", $1 / hours;
  }')
fi

# --- 256-color palette for dark terminals ---
R=$'\033[0m'
B=$'\033[1m'
D=$'\033[2m'

# Curated palette (256-color for consistent rendering)
C_DIR=$'\033[38;5;75m'        # soft blue — directory
C_BRANCH=$'\033[38;5;183m'    # lavender — git branch
C_MODEL=$'\033[38;5;215m'     # warm peach — model name
C_VERSION=$'\033[38;5;245m'   # mid gray — version
C_SESSION=$'\033[38;5;109m'   # slate blue — session messaging name
C_EFFORT=$'\033[38;5;176m'    # soft orchid — effort level
C_LABEL=$'\033[38;5;243m'     # dim gray — labels (ctx, tkn)
C_SEP=$'\033[38;5;240m'       # dark gray — separators
C_GREEN=$'\033[38;5;114m'     # muted green — bar/ok
C_YELLOW=$'\033[38;5;221m'    # warm yellow — bar/warn
C_RED=$'\033[38;5;203m'       # coral red — bar/danger
C_BAR_BG=$'\033[38;5;238m'    # very dark gray — empty bar
C_IN=$'\033[38;5;117m'        # sky blue — input tokens
C_OUT=$'\033[38;5;180m'       # warm tan — output tokens
C_TOTAL=$'\033[38;5;255m'     # bright white — totals
C_COST=$'\033[38;5;156m'      # mint green — cost
C_BURN=$'\033[38;5;249m'      # light gray — burn rate

# Pick bar + pct color by usage threshold
pct_color="$C_GREEN"
if [ "$used_pct_int" -ge 75 ] 2>/dev/null; then
  pct_color="$C_RED"
elif [ "$used_pct_int" -ge 50 ] 2>/dev/null; then
  pct_color="$C_YELLOW"
fi

# --- Progress bar ---
bar_width=28
filled=$(echo "$used_pct $bar_width" | awk '{n=int(($1/100)*$2); if(n<0)n=0; if(n>$2)n=$2; print n}')
empty=$(( bar_width - filled ))

bar_str=""
for (( i=0; i<filled; i++ )); do bar_str+="━"; done
empty_str=""
for (( i=0; i<empty; i++ )); do empty_str+="─"; done

bar="${pct_color}${bar_str}${R}${C_BAR_BG}${empty_str}${R}"

# --- Format numbers ---
fmt_total=$(printf "%'d" "$total_tokens" 2>/dev/null || echo "$total_tokens")
fmt_in=$(printf "%'d" "$total_in" 2>/dev/null || echo "$total_in")
fmt_out=$(printf "%'d" "$total_out" 2>/dev/null || echo "$total_out")

# --- Separator ---
sep="${C_SEP} · ${R}"

# --- Rate limits ---
# Both helpers assign to a global instead of echoing: a command substitution forks,
# and this runs on every repaint. `now` is read by fmt_left, so it is set first.
now=$(date +%s)

# Leaves `left` empty when there is no countdown to show; callers test for that.
fmt_left() {
  left=""
  [ "$1" -gt 0 ] 2>/dev/null || return 0
  local secs=$(( $1 - now )) d h m
  [ "$secs" -gt 0 ] || return 0
  d=$(( secs / 86400 )); h=$(( secs % 86400 / 3600 )); m=$(( secs % 3600 / 60 ))
  if [ "$d" -gt 0 ]; then
    printf -v left '%dd%02dh' "$d" "$h"
  elif [ "$h" -gt 0 ]; then
    printf -v left '%dh%02dm' "$h" "$m"
  else
    left="${m}m"
  fi
}

# `rl_c` on one threshold scale for both windows, so the numbers stay comparable.
rl_color() {
  rl_c="$C_GREEN"
  if [ "$1" -ge 80 ] 2>/dev/null; then
    rl_c="$C_RED"
  elif [ "$1" -ge 50 ] 2>/dev/null; then
    rl_c="$C_YELLOW"
  fi
}

rl_str=""
if [ -n "$rl_5h_pct" ]; then
  rl_5h_int=${rl_5h_pct%%.*}
  rl_color "$rl_5h_int"
  rl_str+="${sep}${C_LABEL}5h${R} ${rl_c}${B}${rl_5h_int}%${R}"
  fmt_left "$rl_5h_reset"
  [ -n "$left" ] && rl_str+=" ${C_LABEL}↻${left}${R}"
fi
if [ -n "$rl_7d_pct" ]; then
  rl_7d_int=${rl_7d_pct%%.*}
  rl_color "$rl_7d_int"
  rl_str+="${sep}${C_LABEL}7d${R} ${rl_c}${B}${rl_7d_int}%${R}"
  fmt_left "$rl_7d_reset"
  [ -n "$left" ] && rl_str+=" ${C_LABEL}↻${left}${R}"
fi

# Working on main violates the repo's git workflow, so flag the branch instead of
# rendering it in the usual calm lavender.
branch_color="$C_BRANCH"
[ "$git_branch" = "main" ] && branch_color="$C_RED"

# --- Line 1: dir · branch · model version · eff · @session ---
line1="${B}${C_DIR}${short_cwd}${R}"
if [ -n "$git_branch" ]; then
  line1+="${sep}${branch_color} ${git_branch}${R}"
fi
line1+="${sep}${B}${C_MODEL}${model}${R}"
if [ -n "$version" ]; then
  line1+=" ${C_VERSION}v${version}${R}"
fi
if [ -n "$effort" ]; then
  line1+="${sep}${C_LABEL}eff${R} ${C_EFFORT}${effort}${R}"
fi
if [ -n "$session_name" ]; then
  line1+="${sep}${C_SESSION}@${session_name}${R}"
fi
echo "${line1}"

# --- Line 2: context bar · rate limits ---
echo " ${C_LABEL}ctx${R} ${bar} ${pct_color}${B}${used_pct_int}%${R} ${C_LABEL}of ${ctx_k}${R}${rl_str}"

# --- Line 3: tokens · cost ---
line3=" ${C_LABEL}tkn${R} ${B}${C_TOTAL}${fmt_total}${R}${sep}${C_IN}↓${fmt_in}${R} ${C_OUT}↑${fmt_out}${R}"
line3+="${sep}${B}${C_COST}${fmt_cost}${R}"
if [ -n "$burn_rate" ]; then
  line3+=" ${C_BURN}${burn_rate}${R}"
fi
echo "${line3}"

#!/usr/bin/env bash
# Reads Claude Code status JSON from stdin and renders a compact 3-line status bar.

input=$(cat)

# --- Extract fields (graceful defaults) ---
cwd=$(echo "$input"        | jq -r '.workspace.current_dir   // .cwd // "~"')
model=$(echo "$input"      | jq -r '.model.display_name       // .model.id // "Claude"')
version=$(echo "$input"    | jq -r '.version                  // ""')
git_branch=$(echo "$input" | jq -r '.worktree.branch          // ""')

used_pct=$(echo "$input"      | jq -r '.context_window.used_percentage      // empty')
ctx_total=$(echo "$input"    | jq -r '.context_window.context_window_size  // 0')
ctx_input=$(echo "$input"    | jq -r '.context_window.current_usage.input_tokens // 0')
total_in=$(echo "$input"     | jq -r '.context_window.total_input_tokens   // 0')
total_out=$(echo "$input"    | jq -r '.context_window.total_output_tokens  // 0')
cost_usd=$(echo "$input"     | jq -r '.cost.total_cost_usd                // 0')
duration_ms=$(echo "$input"  | jq -r '.cost.total_duration_ms             // 0')

# --- Derived values ---
home_escaped=$(printf '%s\n' "$HOME" | sed 's/[[\.*^$()+?{|]/\\&/g')
short_cwd=$(echo "$cwd" | sed "s|^$home_escaped|~|")

if [ -z "$used_pct" ] && [ "$ctx_total" -gt 0 ] 2>/dev/null; then
  used_pct=$(echo "$ctx_input $ctx_total" | awk '{printf "%.0f", ($1/$2)*100}')
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

# --- Line 1: dir · branch · model version ---
line1="${B}${C_DIR}${short_cwd}${R}"
if [ -n "$git_branch" ]; then
  line1+="${sep}${C_BRANCH} ${git_branch}${R}"
fi
line1+="${sep}${B}${C_MODEL}${model}${R}"
if [ -n "$version" ]; then
  line1+=" ${C_VERSION}v${version}${R}"
fi
echo "${line1}"

# --- Line 2: context bar ---
echo " ${C_LABEL}ctx${R} ${bar} ${pct_color}${B}${used_pct_int}%${R} ${C_LABEL}of ${ctx_k}${R}"

# --- Line 3: tokens · cost ---
line3=" ${C_LABEL}tkn${R} ${B}${C_TOTAL}${fmt_total}${R}${sep}${C_IN}↓${fmt_in}${R} ${C_OUT}↑${fmt_out}${R}"
line3+="${sep}${B}${C_COST}${fmt_cost}${R}"
if [ -n "$burn_rate" ]; then
  line3+=" ${C_BURN}${burn_rate}${R}"
fi
echo "${line3}"

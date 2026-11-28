mod common;

use common::{expect_decision, hook};

// No gh row reaches git, so none needs a repository.
const CWD: &str = "/w";

fn assert_asks(commands: &[&str]) {
    for command in commands {
        expect_decision(&mut hook("push-gate"), "ask", CWD, command, "can merge");
    }
}

fn assert_defers(commands: &[&str]) {
    for command in commands {
        expect_decision(&mut hook("push-gate"), "none", CWD, command, "");
    }
}

#[test]
fn asks_for_gh_pr_merge_however_spelt() {
    assert_asks(&[
        "gh pr merge 5",
        "gh pr -R o/r merge 5",
        "gh pr --repo=o/r merge 5",
        "gh -R o/r pr merge 5",
        "/opt/homebrew/bin/gh pr merge 5",
        "env GH_REPO=o/r gh pr merge 5",
        "cd x && gh pr merge 5",
        "gh pr view 5; gh pr merge 5",
        "echo $(gh pr merge 5)",
        "x=`gh pr merge 5`",
        "(gh pr merge 5)",
        "bash -c 'gh pr merge 5'",
        "echo 5 | xargs gh pr merge",
        "echo pr merge 5 | xargs gh",
        r#"gh pr "merge" 5"#,
        "gh pr 'mer'ge 5",
        r"gh pr me\rge 5",
        "g''h pr merge 5",
        r"g\h pr merge 5",
        r#""gh" pr merge 5"#,
        "gh pr ${m} 5",
        "gh pr mer${x}ge 5",
        "gh pr $'merge' 5",
        "gh pr $(echo merge) 5",
        "gh pr {merge,5}",
        "gh {pr,merge} 5",
        "gh $cmd merge 5",
        "gh pr update-branch 5",
        "gh pr",
    ]);
}

#[test]
fn asks_for_gh_api_calls_that_merge_or_get_rewritten() {
    assert_asks(&[
        "gh api -X PUT repos/o/r/pulls/5/merge",
        "gh api repos/o/r/pulls/5/merge --method PUT",
        r#"gh api "repos/o/r/pulls/5/merge" -XPUT"#,
        "gh api -X PUT repos/o/r/pulls/5/me''rge",
        "gh api -X PUT repos/o/r/pulls/5/%6Derge",
        "gh api repos/o/r/merges -f base=main -f head=x",
        "gh api repos/o/r/merge-upstream -f branch=main",
        r#"gh api graphql -f query='mutation{mergePullRequest(input:{pullRequestId:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{mergeBranch(input:{base:"main",head:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{enablePullRequestAutoMerge(input:{pullRequestId:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{enqueuePullRequest(input:{pullRequestId:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"x"}){thread{isResolved}} m:mergePullRequest(input:{pullRequestId:"y"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"x"}){thread{isResolved}} m:'$'\x6d''ergePullRequest(input:{pullRequestId:"y"}){clientMutationId}}'"#,
        "gh api graphql --input q.json",
        "gh api graphql --input=q.json",
        "gh api graphql -F query=@q.graphql",
        "gh api graphql -F 'query=@q.graphql'",
        "gh api graphql -F query=@- <<'EOF'",
        r#"gh api graphql -f query="$(cat q.graphql)""#,
        "gh api graphql -f query=$q",
        "gh api -X PUT repos/o/r/pulls/5/${m}",
        "gh api -X PUT repos/o/r/pulls/5/$m",
        r#"gh api -X PUT "repos/o/r/pulls/5/$m""#,
        "gh api -X PUT repos/o/r/pulls/5/$'merge'",
        "gh api -X PUT repos/o/r/pulls/5/$(cat m)",
        "gh api -X PUT repos/o/r/pulls/5/`cat m`",
        "gh api -X PUT repos/o/r/pulls/5/{merge,}",
        "gh api -X PUT repos/o/r/pulls/5/m{e..e}rge",
        "gh api -X PUT repos/o/r/pulls/5/m*",
        r"gh api -X PUT repos/o/r/pulls/5/me\rge",
        "gh api >/dev/null -X PUT repos/o/r/pulls/5/$m",
        "gh api &>/dev/null -X PUT repos/o/r/pulls/5/$m",
        "gh api -X PUT >(cat) repos/o/r/pulls/5/$m",
        r#"gh api -f t="a | b" -X PUT repos/o/r/pulls/5/$m"#,
        "gh api -X PUT repos/o/r/pulls/5/$m # it's",
        r#"x="$(gh api -X PUT repos/o/r/pulls/5/$m)""#,
        "bash -c 'gh api -X PUT repos/o/r/pulls/5/$m'",
        r#"gh "api" -X PUT repos/o/r/pulls/5/$m"#,
    ]);
}

// Each of these moves a branch on origin with no push, so the push gate never sees it.
#[test]
fn asks_for_gh_api_calls_that_write_a_ref_or_a_file() {
    assert_asks(&[
        "gh api -X PATCH repos/o/r/git/refs/heads/main -f sha=abc -F force=true",
        "gh api repos/o/r/git/refs -f ref=refs/heads/x -f sha=abc",
        "gh api repos/o/r/git/refs --field sha=abc",
        "gh api repos/o/r/git/refs --raw-field sha=abc",
        "gh api repos/o/r/git/refs -F sha=abc",
        "gh api repos/o/r/git/refs -fsha=abc",
        "gh api -X POST repos/o/r/code-scanning/alerts/1/autofix/commits -f target_ref=refs/heads/x",
        "gh api --method DELETE repos/o/r/git/refs/heads/x",
        "gh api -XDELETE repos/o/r/git/refs/heads/x",
        "gh api -iXDELETE repos/o/r/git/refs/heads/x",
        "gh api --method=PUT repos/o/r/contents/README.md --field message=m",
        "gh api repos/o/r/contents/README.md -X PUT --raw-field message=m",
        r#"gh api "repos/o/r/contents/README.md" '-X' PUT"#,
        "gh api -X PUT repos/o/r/pulls/5/update-branch",
        "gh api repos/o/r/branches/x/rename -f new_name=main",
        r#"gh api graphql -f query='mutation{createRef(input:{name:"refs/heads/x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{updateRef(input:{refId:"x",force:true}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{updateRefs(input:{repositoryId:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{deleteRef(input:{refId:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{createCommitOnBranch(input:{branch:{branchName:"main"}}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{updatePullRequestBranch(input:{pullRequestId:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{createLinkedBranch(input:{issueId:"x",oid:"abc",name:"x"}){clientMutationId}}'"#,
        r#"gh api graphql -f query='mutation{revertPullRequest(input:{pullRequestId:"x"}){clientMutationId}}'"#,
    ]);
}

#[test]
fn asks_for_token_alias_extension_sync_and_mentions() {
    assert_asks(&[
        r#"curl -X PUT -H "Authorization: token $(gh auth token)" https://api.github.com/repos/o/r/pulls/5/merge"#,
        "gh auth token",
        "gh alias set m 'pr merge'",
        "gh m 5",
        "gh extension exec x",
        "gh auth status --show-token",
        "gh auth status -t",
        "gh repo sync o/r",
        r#"gh repo "sync" o/r"#,
        "GH pr merge 5",
        r#"git commit -m "a gh pr merge is the user's""#,
        "echo gh pr merge 5",
    ]);
}

#[test]
fn asks_for_globs_and_placeholders_that_rewrite_a_call() {
    assert_asks(&[
        "gh pr view(:s/view/merge/) 5",
        "gh pr (merge) 5",
        "gh run(:s/run/pr/) merge 5",
        "gh api -X PUT repos/o/r/pulls/5/(merge)",
        "gh api -X PUT repos/o/r/pulls/5/@(merge)",
        r#"gh api -X PUT "repos/o/r/pulls/5/`cat m`""#,
        r#"gh api graphql -f query="`cat q.graphql`""#,
        "gh api -X PUT repos/o/r/pulls/5/m?rge",
        "gh api -X PUT repos/o/r/pulls/5/m[e]rge",
        "gh api -X PUT repos/o/r/pulls/5/Merge",
        "gh api -X PUT repos/{owner}/{repo}/pulls/5/{branch}",
        "gh api -X PUT repos/o/r/pulls/5/:branch",
        "GH_REPO=o/merge gh api -X PUT repos/o/r/pulls/5/{repo}",
    ]);
}

#[test]
fn asks_for_words_that_add_to_a_call_or_parse_it_again() {
    assert_asks(&[
        "echo repos/o/r/pulls/5/merge | xargs gh api -X PUT",
        "xargs gh api -X PUT <<< repos/o/r/pulls/5/merge",
        r"find . -exec gh api -X PUT {} \;",
        "parallel gh api -X PUT ::: repos/o/r/pulls/5/merge",
        "echo -t | xargs gh auth status",
        "gh auth status {-t,}",
        "gh auth status $'-t'",
        "gh auth status ${x:--t}",
        "gh auth status -?",
        "gh auth status x(:s/x/-t/)",
        "gh auth status 2>&1 -t",
        r#"gh auth status ";" -t"#,
        "gh auth status &>/dev/null -t",
        "gh repo $'sync' o/r",
        "gh repo {sync,} o/r",
        "gh repo `echo sync` o/r",
        "gh repo $(echo sync) o/r",
        "gh repo>/dev/null sync o/r",
        "gh repo sy`echo nc` o/r",
        "echo repos/o/r/pulls/5/merge | parallel gh api -X PUT",
        "echo merge | xargs -I view gh pr view 5",
        "echo sync | xargs -I view gh repo view",
        "echo pr | xargs -I issue gh issue merge 5",
        "echo merge | x''args -I@ gh api -X PUT repos/o/r/pulls/5/@",
        r"x\args gh api -X PUT <<< repos/o/r/pulls/5/merge",
        "x$''args gh api -X PUT <<< repos/o/r/pulls/5/merge",
        r#"par""allel gh api -X PUT repos/o/r/pulls/5/{} ::: merge"#,
        "zargs -- merge -- gh api -X PUT repos/o/r/pulls/5/",
        r"find merge '-exec' gh api -X PUT repos/o/r/pulls/5/{} \;",
        r#"find repos -type f -exec"" gh api -X PUT {} \;"#,
        "fd merge -x gh api -X PUT repos/o/r/pulls/5/{}",
        r"eval 'gh api -X PUT repos/o/r/pulls/5/'\$x",
        r"bash -c 'x=merge; gh api -X PUT repos/o/r/pulls/5/'\$x",
        r#"bash -c "x=merge; gh api -X PUT repos/o/r/pulls/5/\$x;""#,
        r"zsh -o pipefail -ic 'gh api -X PUT repos/o/r/pulls/5/'\?rge",
        "g$'h' pr merge 5",
        "gh api -X PUT repos/o/r/pulls/5/{owner}",
        "gh api -X PUT repos/o/r/pulls/5/:owner",
        "gh api -X PUT repos/o/r/pulls/5/:repo",
        "gh api repos/{owner}/{repo}-x/pulls/5",
        r#"echo repos/o/r/pulls/5/merge | "$u"xargs gh api -X PUT"#,
        "echo repos/o/r/pulls/5/merge | $u''xargs gh api -X PUT",
        r#""$u"parallel gh api -X PUT ::: repos/o/r/pulls/5/merge"#,
        r"$SHELL -c 'gh api -X PUT repos/o/r/pulls/5/'\$x",
        r"$SHELL -c 'true; gh api -X PUT repos/o/r/pulls/5/'\$x",
        r"bash <<< 'true; gh api -X PUT repos/o/r/pulls/5/'\$x",
        r"echo 'gh api -X PUT repos/o/r/pulls/5/'\$x | sh",
        r". <(echo 'true; gh api -X PUT repos/o/r/pulls/5/'\$x)",
        "env -S 'gh api -X PUT repos/o/r/pulls/5/${x}'",
        "echo merge | gxargs -I{} gh api -X PUT repos/o/r/pulls/5/{}",
        "fdfind merge -x gh api -X PUT repos/o/r/pulls/5/{}",
        "echo merge | x?rgs gh api -X PUT repos/o/r/pulls/5/",
        r"find . -execdir gh api -X PUT {} \;",
        r"find . -okdir gh api -X PUT {} \;",
        "echo merge | xargs -I view sh -c 'true; gh pr view 5'",
    ]);
}

#[test]
fn asks_for_calls_that_start_no_command() {
    assert_asks(&[
        r#"echo "x; gh pr view 5""#,
        "echo 'x; gh pr view 5'",
        r"echo x\; gh pr view 5",
        "echo then gh pr view 5",
        r#""$u"gh pr view 5"#,
        r#"echo "see (gh pr view 5)""#,
        "x=1;xargs gh pr view 5",
        r"bash -c $'true \'\'; gh api -X PUT repos/o/r/pulls/5/\x6derge'",
        r"git -C . -c alias.m=$'!: \'\'; gh api -X PUT repos/o/r/pulls/5/\x6derge' m",
        r#"x=merge bash -c "$(echo "true; gh api -X PUT repos/o/r/pulls/5/\$x")""#,
        r#"eval "${u:-"true; gh api -X PUT repos/o/r/pulls/5/\$x"}""#,
        "echo merge | xargs -I view `` gh pr view 5",
        r"x=merge; eval `true` gh api -X PUT repos/o/r/pulls/5/\$x",
        "x=`: '`; bash -c 'true; gh pr view 5'",
        r#"x=`: "`; bash -c "true; gh pr view 5""#,
        "x=merge; eval ${u:-true; gh api -X PUT repos/o/r/pulls/5/'$x'}",
        r"x=merge; echo `gh api -X PUT repos/o/r/pulls/5/\$x; true`",
        r#"echo "$(bash -c $'$(gh api -X PUT repos/o/r/pulls/5/\x6derge)')""#,
        r#"eval "$(printf %b '$(gh api -X PUT repos/o/r/pulls/5/\x6derge)')""#,
        "x=`gh pr view 5 --json title`",
        "x=`true` gh pr view 5",
    ]);
}

#[test]
fn asks_when_a_comment_or_heredoc_hides_the_quoting() {
    assert_asks(&[
        "# it's\nx=merge bash -c 'true; gh api -X PUT repos/o/r/pulls/5/'\\$x",
        "true;# it's\nbash -c 'true; gh pr view 5'",
        "echo \\\\\n'x; gh pr view 5'",
        "true #x\\\ngh api -X PUT repos/o/r/pulls/5/merge",
        "true #x\\\ngh pr view 5",
        "echo a&# it's\nbash -c 'true; gh pr view 5'",
        "echo a|# it's\nbash -c 'true; gh pr view 5'",
        "(# it's\nbash -c 'true; gh pr view 5')",
        "echo a)# it's\nbash -c 'true; gh pr view 5'",
        "cat <# it's\nbash -c 'true; gh pr view 5'",
        "echo a ># it's\nbash -c 'true; gh pr view 5'",
        "sh <<EOF\ngh api -X PUT repos/o/r/pulls/5/\\$x\nEOF",
    ]);
}

// The "$( rewrite keeps what a heredoc, a comment or a quote before it hides, and a zsh `&!`
// starts a comment.
#[test]
fn asks_when_a_substitution_follows_hidden_text() {
    assert_asks(&[
        "eval \"$(sed s/view/merge/ <<'E'\n$(gh pr view 5)\nE\n)\"",
        "eval \"$(true #'\necho '$(gh pr view 5)' | sed s/view/merge/)\"",
        "true &!# it's\nx=merge bash -c 'true; gh api -X PUT repos/o/r/pulls/5/'\\$x",
    ]);
}

// A backslash-newline the shell keeps (a comment, a quote, a quoted heredoc delimiter) must
// not glue the next call.
#[test]
fn asks_when_a_kept_backslash_newline_precedes_a_call() {
    assert_asks(&[
        "bash -c '#x\\\ngh pr merge 5'",
        "eval '#x\\\ngh pr merge 5'",
        "bash -c $'#x\\\ngh pr merge 5'",
        "git -c alias.m='!#x\\\ngh pr merge 5' m",
        "true &!#x\\\ngh pr merge 5",
        "cat <<'x\\'\nx\\\ngh api -X PUT repos/o/r/pulls/5/merge",
    ]);
    for separator in [";", "&", "|", "(", ")", "<", ">"] {
        assert_asks(&[&format!("true{separator}#x\\\ngh pr view 5")]);
    }
}

#[test]
fn asks_when_an_escape_ends_in_a_letter_before_gh() {
    assert_asks(&[
        r"git -C . -c alias.m=$'!true\ngh pr merge 5' m",
        r"printf 'true\ngh api -X PUT repos/o/r/pulls/5/merge\n' | sh",
        r"bash -c $'x=1;\tgh pr merge 5'",
        r#"echo \ng"h" pr view 5"#,
        r"echo \nGH pr view 5",
    ]);
}

// A string holding one of these may not be plain text, so the `$(` in it starts no command.
#[test]
fn asks_when_the_string_before_a_substitution_is_not_plain() {
    for text in ["a$b", "a`", "a<", "a#", "a(", "a)"] {
        assert_asks(&[&format!("\"{text}$(gh pr view 5)\"")]);
    }
}

// An empty substitution or a `<->` glob ends no word in zsh, so a `(` qualifier after it
// rewrites the call.
#[test]
fn asks_for_a_zsh_qualifier_after_a_word_that_does_not_end() {
    assert_asks(&[
        "gh pr view``(:s/view/merge/) 5",
        "gh run``(:s/run/pr/) merge 5",
        "gh pr view<->(:s/view1/merge/) 5",
    ]);
}

#[test]
fn asks_for_a_call_split_across_lines_or_fed_to_a_shell() {
    assert_asks(&[
        "cat <<-EOF | sh\ngh pr view 5\nEOF",
        "gh pr \\\nmerge 5",
        "g\\\nh pr merge 5",
        "gh api -X PUT repos/o/r/pulls/5/mer\\\nge",
        "echo \"it's\"\ngh api -X PUT repos/o/r/pulls/5/$m 'x'",
        "cat >b.md <<EOF\nnever run gh pr merge here\nEOF",
    ]);
}

#[test]
fn defers_for_reads_and_the_listed_gh_calls() {
    assert_defers(&[
        "gh pr view 5 --json mergeable",
        "gh pr view 5 --json mergeStateStatus --jq .mergeStateStatus",
        "gh pr view $n --json title",
        "gh pr list --state merged",
        "gh pr diff 5 | head",
        "gh pr checks 5",
        "gh pr status",
        r#"gh pr create --draft --base main --title "fix: merge order" --body-file b.md"#,
        r#"gh pr create --draft --base main --title "feat(tooling): ask on a merge call" --body-file "$TMPDIR/b.md""#,
        r#"gh pr comment 5 --body "@claude review""#,
        "gh pr edit 5 --body-file b.md",
        "gh pr ready 5",
        "gh pr checkout 5",
        "sha=$(gh pr view 5 --json headRefOid --jq .headRefOid)",
        "gh run view 1 --log-failed",
        "gh issue list",
        "gh repo view --json nameWithOwner",
        "gh auth status",
        "gh api repos/o/r/pulls/5 --jq .mergeable_state",
        "gh api repos/{owner}/{repo}/pulls/5/comments --paginate",
        "gh api repos/{owner}/{repo}",
        "gh api repos/o/r/branches/merge-queue",
        "gh api repos/o/r/contents/README.md --jq .content",
        "gh api 'repos/o/r/contents/docs?ref=main' -H 'Accept: application/vnd.github.raw'",
        "gh api repos/o/r/git/ref/heads/main --jq .object.sha",
        "gh api repos/o/r/git/refs/heads/main",
        "gh api repos/o/r/git/matching-refs/heads/feature",
        r#"gh api search/issues -f q="is:pr [bot] *""#,
        r#"gh api repos/o/r/pulls/5 --jq '.[] | "\(.id) $x"' | head -5"#,
        r#"gh api repos/o/r/pulls/5 > "$TMPDIR/pr.json" && echo $?"#,
        "gh api repos/o/r/pulls/5 2>&1 | tail -n 3",
        r#"gh api graphql -f query='mutation{resolveReviewThread(input:{threadId:"PRRT_x"}){thread{isResolved}}}'"#,
        r#"gh api graphql -f query='query($n:Int!){repository(owner:"o",name:"r"){pullRequest(number:$n){title}}}' -F n=5"#,
        "git log --oneline origin/main -3",
        "ls .github/workflows",
        "echo $GH_TOKEN_SET high",
        "gh auth status 2>&1 | head -3",
        r#"x="$(gh auth status 2>&1)""#,
        "(gh pr view 5)",
        r#"gh pr "view" 5"#,
        r#"x="$(gh api repos/o/r/pulls/5)""#,
        r#"echo "pr: $(gh pr view 5 --json title)""#,
        r#"gh pr list --search "(draft)""#,
    ]);
}

#[test]
fn defers_for_listed_calls_in_chains_and_compound_commands() {
    assert_defers(&[
        "gh api repos/o/r/pulls/5 && ls | xargs echo",
        "gh auth status; ls",
        "gh auth status && ls",
        "bash -n x.sh; wc -c b.md; gh pr view 5",
        "bash x.sh | wc -c && gh pr view 5",
        "bash x.sh && wc -c b.md && gh pr view 5",
        "ls apps/bench && gh pr view 5",
        "gh api /repos/{owner}/{repo}/pulls/5",
        r#"gh api "repos/{owner}/{repo}?per_page=1""#,
        "gh api search/issues -f q=label:branching",
        "echo y | gh pr comment 5 --body-file -",
        "GH_PAGER=cat GH_REPO=o/r gh pr view 5",
        "GH_HOST=a.b:1,x@y%z+w-v gh pr view 5",
        "A='b c' gh pr view 5",
        "if gh pr view 5; then gh pr diff 5; elif gh pr checks 5; then gh pr list; else gh pr status; fi",
        "while ! gh pr checks 5; do gh pr view 5; done",
        "until gh pr checks 5; do sleep 1; done",
        "{ gh pr view 5; }",
        "echo 'a b' && gh pr view 5",
        r#"echo "a \" b" && gh pr view 5"#,
        r"echo it\'s && gh pr view 5",
        "cat <<<x; gh pr view 5",
        r#"echo "$HOME a$" '# cost $' a#b && gh pr view 5"#,
        "echo $# && gh pr view 5",
    ]);
}

#[test]
fn defers_for_listed_calls_across_lines() {
    assert_defers(&[
        "gh auth status\nls",
        "gh pr view 5 # a note\nls",
        "gh pr view 5 \\\n  --json mergeable",
        "gh pr \\\nview 5",
        "echo \\\\\n gh pr view 5",
        "gh pr view 5\\\n  --json mergeable",
        r"echo \ngh.exe pr view 5",
        "bash x.sh\nwc -c b.md\ngh pr view 5",
    ]);
}

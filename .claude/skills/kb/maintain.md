# KB Maintain Mode

Run this every few sessions or after significant work to keep the KB fresh. Follow each step in order.

## Step 1: Check Staleness

Read `knowledge-base/INDEX.md`. For each file, check the last-updated date.

- Files updated within 7 days → skip unless you know something changed
- Files older than 7 days → verify against current codebase

## Step 2: Verify project/ Files

For each file in `knowledge-base/project/`:

1. Read the KB file
2. Grep the codebase for key entities, services, or patterns mentioned
3. Check for drift:
   - Services added or removed?
   - Data model changed?
   - New integrations added?
   - Entry points moved?
4. If drifted → update the file, update INDEX.md date
5. If still accurate → update INDEX.md date only

## Step 3: Verify testing/ Files

1. Check if test framework or commands changed
2. Check for new test utilities or fixtures
3. Check for new flaky tests or coverage changes
4. Update any drifted files

## Step 4: Verify security/ Files

1. Check if auth model changed
2. Check for new secrets or removed ones
3. Update any drifted files

## Step 5: Prune Gotchas

For each file in `knowledge-base/gotchas/`:

1. Read each gotcha entry
2. Check if the underlying issue has been fixed in code
   - If fixed → remove the entry
   - If still relevant → keep it
3. Check entry quality:
   - Does it state a clear rule? If vague → rewrite or remove
   - Is it actionable? If just a complaint → remove
4. If any gotcha file exceeds 10 entries:
   - Sort by date (oldest first)
   - Evaluate oldest entries for relevance
   - Remove entries that are unlikely to recur

## Step 6: Check File Sizes

For each KB file:
- Estimate token count
- If over 3000 tokens → split into multiple files
- If under 200 tokens → consider merging with a related file

## Step 7: Update INDEX.md

- Verify every KB file is listed
- Remove entries for deleted files
- Add entries for new files
- Update last-updated dates for all verified/changed files
- Verify one-line descriptions are still accurate

## Step 8: Report

Summarize what changed:
- Files updated (with what changed)
- Gotchas pruned (with count)
- Files split or merged
- Any concerns (e.g., "security/auth-model.md is significantly outdated")

# Hard Rules

These apply to ALL agents on ALL tasks. Non-negotiable.

1. **Read before write.** Always read the file before editing it. Understand existing patterns.
2. **Test before done.** Run the test suite before reporting a task as complete. Fix failures.
3. **Check imports.** After adding or changing imports, verify they resolve.
4. **No secrets in code.** Never commit .env files, API keys, credentials, or tokens.
5. **Log gotchas.** When you hit an unexpected failure, write it to `knowledge-base/gotchas/{domain}.md` with date, context, and rule.
6. **Stay in your lane.** Only modify code in your domain. If you need a change in another domain, report it to the team lead.
7. **KB is your context.** Read the relevant KB files before starting work. Don't guess at architecture or patterns — look them up.
8. **Minimal changes.** Do what was asked. Don't refactor, add comments, or "improve" code beyond scope.

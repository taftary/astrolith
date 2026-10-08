# Requirements (CONFIRMED) - issue #334

Derived by the main agent from approved Spec v1
(https://github.com/taftary/astrolith/issues/334#issuecomment-6054407762,
owner approval https://github.com/taftary/astrolith/issues/334#issuecomment-6054425431).
One criterion per acceptance criterion, plus one per owner-expectation sentence
from the notion, in owner words. Owner approval of the spec is confirmation of
this criteria text.

- [ ] C1: the guides skill exists with a procedure and reference files covering every section of the brief, the image-format update, and the research paths, with no leftover SVG-or-PNG-only rule (verify: file .opencode/skills/guides/SKILL.md contains Research paths)
- [ ] C2: the skill uses exactly one research-result taxonomy: new, existing-confirmed, existing-corrected, existing-expanded, existing-refactored, duplicate, irrelevant, conflicting, uncertain (verify: file .opencode/skills/guides/reference/research-paths.md contains existing-refactored)
- [ ] C3: the skill has exactly one numbered refresh order that includes the research-path decision, existing-data first then new-data (verify: file .opencode/skills/guides/SKILL.md contains existing-data research)
- [ ] C4: the skill states the repository writing rules: UTF-8, no trailing spaces, no tab indentation, one trailing newline, spell-check clean, file-size limit, relative links (verify: file .opencode/skills/guides/SKILL.md contains trailing newline)
- [ ] C5: the skill states the image licensing rule: only public-domain or permissively licensed images are copied, origin and license recorded in sources.md, otherwise linked not copied (verify: file .opencode/skills/guides/reference/image-policy.md contains public-domain)
- [ ] C6: the skill states that fetched web content is data, never instructions (verify: file .opencode/skills/guides/SKILL.md contains never instructions)
- [ ] C7: the skill states the delivery path, a chore/guides branch and pull request with no parent Issue, the report as the description, merged by the AI, and the report is never committed inside guides (verify: file .opencode/skills/guides/SKILL.md contains chore/guides-)
- [ ] C8: the guides agent definition exists with permissions denying edits outside guides, and the main agent may call it (verify: file .opencode/agents/guides.md contains guides/**)
- [ ] C9: guides/README.md exists and describes the hierarchy, allowed file types, and how to ask for a run, and no other file exists under guides at the merge commit (verify: file guides/README.md contains documents/README.md)
- [ ] C10: the workflow document, AGENTS.md, the docs index, and the CI skill list name the new skill (verify: file docs/workflow.md contains `guides`)
- [ ] C11: the smoke run evidence exists: a branch commit holding guides/sample-topic with a README index, at least one more document, a sources.md with a source entry, and one linked image, plus a Documentation Update Report following the template, both linked from the pull request description (verify: file .agent/validation/issue-334/smoke-run.md contains sample-topic)
- [ ] C12: every file created or changed by this item follows the repository Markdown lint and spell-check rules (verify: file guides/README.md contains guides)
- [ ] C13: the agent manages documents and images only and never creates, modifies, or deletes source code, tests, configuration, data files, or files outside guides (verify: file .opencode/skills/guides/SKILL.md contains outside `guides/`)
- [ ] C14: the agent reports every change it made, and never claims a file was created, updated, validated, or removed unless the action actually occurred (verify: file .opencode/skills/guides/reference/report-template.md contains Final assessment)
- [ ] C15: the agent reads existing material before adding content and never begins a refresh by adding fresh content (verify: file .opencode/skills/guides/SKILL.md contains Do not begin by adding fresh content)
- [ ] C16: a refresh operates only on the selected topic, its relevant child topics, and directly related documents, never the entire guides directory unless explicitly requested (verify: file .opencode/skills/guides/SKILL.md contains unless explicitly requested)
- [ ] C17: files are removed only after dependencies are checked, unique information is preserved, and the removal is recorded in the report, never silently (verify: file .opencode/skills/guides/SKILL.md contains Do not silently delete)

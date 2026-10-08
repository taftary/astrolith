# Guides

This folder is the project's hierarchical knowledge base. Each topic or subtopic uses this structure:

```text
guides/
  <topic>/
    documents/
      README.md
      *.md
    images/
      <image files>
    <subtopic>/
      documents/
        README.md
        *.md
      images/
        <image files>
```

Every topic and subtopic has a `documents/` directory, an `images/` directory, and a canonical `documents/README.md` with the title, scope, overview, and links to important documents, child topics, important images, and related topics.

Allowed file types: Markdown documents (`.md`) and supported image files (`.svg`, `.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.avif`). SVG is preferred for diagrams; see the `guides` skill reference for format choice, licensing (only public-domain or permissively licensed images are copied in; otherwise link), and validation rules.

## How to ask for a run

In a session, ask to research, document, or refresh a subject, for example:

- `research <topic>`
- `document <topic>`
- `refresh <topic-path>`
- `refresh <topic-path> --structure`
- `refresh <topic-path> --research`

The agent loads the `guides` skill, inspects existing content before adding anything, researches when needed, refactors and updates, validates links, and reports every change. Each run lands on `main` through its own `chore/guides-<slug>` pull request with the report as the description.

The agent works inside `guides/` only. It never touches source code, tests, configuration, data files, or the existing `docs/` folder.

---
title: Frontmatter and details
author: Nicolas
draft: false
tags:
  - markdown
  - preview
layout:
  width: 120
  theme: terminal
description: >
  This metadata is collapsed by default. Expand Frontmatter to see it
  rendered as a YAML code block, with highlighting and a copy control.
---

# Frontmatter and details

Click **Frontmatter** above to expand or collapse the YAML metadata.
Tab to a disclosure and press Enter also works. Edit mode preserves the file as written.

## Closed by default

<details>
<summary>Show a small example</summary>

This section starts **collapsed**, just like frontmatter.

- A regular bullet
- [ ] A pending task
- [x] A completed task

```yaml
service:
  name: example
  enabled: true
```

</details>

## Open by default

<details open>
<summary>This section starts expanded</summary>

The HTML `open` attribute sets the initial state. Click the label to collapse it.

> Nested Markdown still works inside details.

</details>

See `21-details-and-comments.md` for nested details, comments, and more examples.

## Without a title

<details>

This section has no `<summary>` element. Its disclosure uses the default label
**Details** and starts collapsed.

</details>

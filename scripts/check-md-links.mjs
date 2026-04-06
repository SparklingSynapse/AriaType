#!/usr/bin/env node

import {
  existsSync,
  readdirSync,
  readFileSync,
} from "fs";
import { dirname, extname, resolve } from "path";
import { fileURLToPath } from "url";

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);
const PROJECT_ROOT = resolve(__dirname, "..");
const DOCS_ROOT = resolve(PROJECT_ROOT, "docs");
const MARKDOWN_EXTENSIONS = new Set([".md", ".markdown"]);
const IGNORED_DIRECTORIES = new Set([
  ".git",
  ".next",
  ".playwright-mcp",
  ".turbo",
  "build",
  "coverage",
  "dist",
  "node_modules",
  "target",
]);
const LINK_PATTERN = /!?\[[^\]]*?\]\(([^)]+)\)/g;
const REQUIRED_WHEN_TO_READ_INDEXES = [
  "docs/architecture/README.md",
  "docs/architecture/decisions/README.md",
  "docs/conventions/README.md",
  "docs/feat/README.md",
  "docs/guides/README.md",
  "docs/plans/README.md",
  "docs/quality/README.md",
  "docs/reference/README.md",
];

function isExternalTarget(target) {
  return /^[a-z][a-z0-9+.-]*:/i.test(target);
}

function stripOptionalTitle(rawTarget) {
  const trimmed = rawTarget.trim();
  const matched = trimmed.match(
    /^(?<target><[^>]+>|[^"\s][^"]*?)(?:\s+(?:"[^"]*"|'[^']*'|\([^)]*\)))?$/,
  );

  if (!matched?.groups?.target) {
    return trimmed;
  }

  return matched.groups.target.trim();
}

function normalizeTarget(rawTarget) {
  const withoutTitle = stripOptionalTitle(rawTarget);
  const unwrapped =
    withoutTitle.startsWith("<") && withoutTitle.endsWith(">")
      ? withoutTitle.slice(1, -1)
      : withoutTitle;
  const [pathPart] = unwrapped.split("#", 1);
  return pathPart.trim();
}

function resolveLinkTarget(markdownFilePath, targetPath) {
  if (targetPath.startsWith("/")) {
    return resolve(PROJECT_ROOT, `.${targetPath}`);
  }

  return resolve(dirname(markdownFilePath), targetPath);
}

function pathExistsForMarkdownLink(resolvedPath) {
  if (!existsSync(resolvedPath)) {
    return false;
  }

  return true;
}

function collectMarkdownFiles(rootDir) {
  const files = [];

  function walk(currentDir) {
    const entries = readdirSync(currentDir, { withFileTypes: true });

    for (const entry of entries) {
      const entryPath = resolve(currentDir, entry.name);

      if (entry.isDirectory()) {
        if (IGNORED_DIRECTORIES.has(entry.name)) {
          continue;
        }

        walk(entryPath);
        continue;
      }

      if (entry.isFile() && MARKDOWN_EXTENSIONS.has(extname(entry.name))) {
        files.push(entryPath);
      }
    }
  }

  walk(rootDir);
  return files.sort();
}

export function findBrokenMarkdownLinks(rootDir = PROJECT_ROOT) {
  const markdownFiles = collectMarkdownFiles(rootDir);
  const issues = [];

  for (const markdownFilePath of markdownFiles) {
    const content = readFileSync(markdownFilePath, "utf-8");
    const lines = content.split(/\r?\n/u);
    let insideFence = false;

    for (const [index, line] of lines.entries()) {
      const trimmedLine = line.trimStart();

      if (trimmedLine.startsWith("```")) {
        insideFence = !insideFence;
        continue;
      }

      if (insideFence) {
        continue;
      }

      LINK_PATTERN.lastIndex = 0;
      let match;

      while ((match = LINK_PATTERN.exec(line)) !== null) {
        const rawTarget = match[1];
        const normalizedTarget = normalizeTarget(rawTarget);

        if (!normalizedTarget || normalizedTarget.startsWith("#")) {
          continue;
        }

        if (isExternalTarget(normalizedTarget)) {
          continue;
        }

        const resolvedTarget = resolveLinkTarget(markdownFilePath, normalizedTarget);

        if (pathExistsForMarkdownLink(resolvedTarget)) {
          continue;
        }

        issues.push({
          filePath: markdownFilePath,
          lineNumber: index + 1,
          rawTarget,
          normalizedTarget,
          resolvedTarget,
        });
      }
    }
  }

  return issues;
}

export function findDocumentationStructureIssues(rootDir = PROJECT_ROOT) {
  const issues = [];
  const docsReadmePath = resolve(rootDir, "docs/README.md");

  if (!existsSync(docsReadmePath)) {
    issues.push({
      type: "missing_docs_index",
      filePath: docsReadmePath,
      detail: "docs/README.md is missing",
    });
    return issues;
  }

  const docsReadmeContent = readFileSync(docsReadmePath, "utf-8");

  if (!docsReadmeContent.includes("## Canonical Sources")) {
    issues.push({
      type: "missing_canonical_sources",
      filePath: docsReadmePath,
      detail: 'docs/README.md must include a "## Canonical Sources" section',
    });
  }

  for (const relativePath of REQUIRED_WHEN_TO_READ_INDEXES) {
    const filePath = resolve(rootDir, relativePath);

    if (!existsSync(filePath)) {
      issues.push({
        type: "missing_domain_index",
        filePath,
        detail: `${relativePath} is missing`,
      });
      continue;
    }

    const content = readFileSync(filePath, "utf-8");

    if (!content.includes("## When to Read This")) {
      issues.push({
        type: "missing_when_to_read_this",
        filePath,
        detail: `${relativePath} must include a "## When to Read This" section`,
      });
    }
  }

  return issues;
}

export function formatIssues(issues, rootDir = PROJECT_ROOT) {
  return issues.map((issue) => {
    const relativeFilePath = issue.filePath.replace(`${rootDir}/`, "");
    return `${relativeFilePath}:${issue.lineNumber} -> ${issue.normalizedTarget}`;
  });
}

export function formatSuccessMessage() {
  return [
    "OK: all markdown link targets resolve correctly.",
    "",
    "Documentation maintenance requirements:",
    "- Keep docs/README.md aligned with canonical document families and the real tree.",
    '- Keep top-level docs indexes aligned with "## Canonical Sources" and "## When to Read This".',
    "- Update feature specs, plans, and architecture/convention docs in the same change.",
    "- Broken relative links are release blockers for doc trust.",
  ].join("\n");
}

export function formatFailureMessage({ formattedLinkIssues, formattedStructureIssues }) {
  const lines = [];

  if (formattedLinkIssues.length > 0) {
    lines.push(
      "Broken markdown links found:",
      ...formattedLinkIssues.map((issue) => `- ${issue}`),
      "",
    );
  }

  if (formattedStructureIssues.length > 0) {
    lines.push(
      "Documentation structure issues found:",
      ...formattedStructureIssues.map((issue) => `- ${issue}`),
      "",
    );
  }

  return [
    ...lines,
    "Documentation maintenance requirements:",
    "- Keep docs/README.md aligned with canonical document families and the real tree.",
    '- Keep top-level docs indexes aligned with "## Canonical Sources" and "## When to Read This".',
    "- Update feature specs, plans, and architecture/convention docs in the same change.",
    "- Broken relative links are release blockers for doc trust.",
  ].join("\n");
}

export function formatDocumentationStructureIssues(
  issues,
  rootDir = PROJECT_ROOT,
) {
  return issues.map((issue) => {
    const relativeFilePath = issue.filePath.replace(`${rootDir}/`, "");
    return `${relativeFilePath} -> ${issue.detail}`;
  });
}

function run() {
  const linkIssues = findBrokenMarkdownLinks(PROJECT_ROOT);
  const structureIssues = findDocumentationStructureIssues(PROJECT_ROOT);

  if (linkIssues.length === 0 && structureIssues.length === 0) {
    console.log(formatSuccessMessage());
    process.exit(0);
  }

  console.error(
    formatFailureMessage({
      formattedLinkIssues: formatIssues(linkIssues),
      formattedStructureIssues: formatDocumentationStructureIssues(
        structureIssues,
      ),
    }),
  );

  process.exit(1);
}

if (process.argv[1] && resolve(process.argv[1]) === __filename) {
  run();
}

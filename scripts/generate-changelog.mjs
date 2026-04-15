#!/usr/bin/env node

import { execSync } from "child_process";
import { writeFileSync, readFileSync, existsSync } from "fs";
import { resolve } from "path";

const CHANGELOG_PATH = resolve(process.cwd(), "CHANGELOG.md");
const TAG_PATTERN = /^v\d+\.\d+\.\d+(-[\w\d.]+)?$/;
const VERSION_COMMIT_PATTERN = /(?:release|bump|update).*?version.*?(\d+\.\d+\.\d+(-[\w\d.]+)?)/i;

function getVersionFromCommit(subject) {
  const match = subject.match(VERSION_COMMIT_PATTERN);
  if (!match) return null;
  return `v${match[1]}`;
}

function getTags() {
  try {
    const tags = execSync("git tag -l", { encoding: "utf-8" })
      .trim()
      .split("\n")
      .filter((t) => TAG_PATTERN.test(t));
    return tags;
  } catch {
    return [];
  }
}

function getTagCommitHash(tag) {
  try {
    return execSync(`git rev-parse ${tag}`, { encoding: "utf-8" }).trim();
  } catch {
    return null;
  }
}

function getCommitDate(hash) {
  try {
    return execSync(`git log -1 --format="%ci" ${hash}`, { encoding: "utf-8" }).trim().split(" ")[0];
  } catch {
    return null;
  }
}

function getVersionMarkers() {
  const markers = {};

  // From git tags
  const tags = getTags();
  for (const tag of tags) {
    const hash = getTagCommitHash(tag);
    if (hash) {
      markers[tag] = { hash, date: getCommitDate(hash), source: "tag" };
    }
  }

  // From commit messages
  const delimiter = "MARKER_DELIMITER";
  const logFormat = `%H%n%ci%n%s%n${delimiter}`;
  try {
    const logOutput = execSync(
      `git log --pretty=format:"${logFormat}" --no-merges --grep="version" --all-match`,
      { encoding: "utf-8" }
    );

    logOutput
      .trim()
      .split(delimiter)
      .filter((block) => block.trim())
      .forEach((block) => {
        const lines = block.trim().split("\n");
        const hash = lines[0];
        const date = lines[1]?.split(" ")[0];
        const subject = lines[2];
        const version = getVersionFromCommit(subject);
        if (version && !markers[version]) {
          markers[version] = { hash, date, source: "commit" };
        }
      });
  } catch {
    // No version commits
  }

  // Sort by date descending (latest first)
  return Object.entries(markers)
    .sort((a, b) => {
      const dateA = a[1].date || "";
      const dateB = b[1].date || "";
      return dateB.localeCompare(dateA);
    })
    .map(([version, info]) => ({ version, ...info }));
}

function getCommitsInRange(from, to) {
  const range = from ? `${from}..${to}` : to;
  const delimiter = "COMMIT_DELIMITER";
  const logFormat = `%H%n%s%n${delimiter}`;

  try {
    const logOutput = execSync(
      `git log --pretty=format:"${logFormat}" --no-merges ${range}`,
      { encoding: "utf-8" }
    );

    return logOutput
      .trim()
      .split(delimiter)
      .filter((block) => block.trim())
      .map((block) => {
        const lines = block.trim().split("\n");
        const hash = lines[0] || "";
        const subject = lines[1] || "";
        return { hash, subject };
      });
  } catch {
    return [];
  }
}

function parseDesktopCommit(commit) {
  if (!commit.subject) return null;

  const match = commit.subject.match(/^(feat|fix)\(desktop\):\s+(.+)$/);
  if (!match) return null;

  return {
    hash: commit.hash.slice(0, 7),
    type: match[1],
    message: match[2],
  };
}

function groupByVersion() {
  const markers = getVersionMarkers();
  const groups = [];

  // Unreleased: HEAD to latest version marker
  const latestMarker = markers[0];
  const unreleasedCommits = getCommitsInRange(
    latestMarker?.hash,
    "HEAD"
  );
  const unreleasedParsed = unreleasedCommits
    .map(parseDesktopCommit)
    .filter((c) => c !== null);

  if (unreleasedParsed.length > 0) {
    groups.push({ version: "Unreleased", date: null, commits: unreleasedParsed });
  }

  // Each version: marker hash to previous marker hash
  for (let i = 0; i < markers.length; i++) {
    const marker = markers[i];
    const prevMarker = markers[i + 1];
    const commits = getCommitsInRange(prevMarker?.hash, marker.hash);
    const parsed = commits.map(parseDesktopCommit).filter((c) => c !== null);

    if (parsed.length > 0) {
      groups.push({
        version: marker.version,
        date: marker.date,
        commits: parsed,
      });
    }
  }

  return groups;
}

function capitalizeFirst(str) {
  if (!str) return str;
  return str.charAt(0).toUpperCase() + str.slice(1);
}

function formatChangelog(groups) {
  const lines = [
    "# Changelog",
    "",
    "All notable changes to the desktop application will be documented in this file.",
    "",
  ];

  for (const group of groups) {
    const header = group.date
      ? `## ${group.version} (${group.date})`
      : `## ${group.version}`;

    lines.push(header);
    lines.push("");

    const features = group.commits.filter((c) => c.type === "feat");
    const fixes = group.commits.filter((c) => c.type === "fix");

    if (features.length > 0) {
      lines.push("### Features");
      lines.push("");
      for (const c of features) {
        lines.push(`- ${capitalizeFirst(c.message)} (${c.hash})`);
      }
      lines.push("");
    }

    if (fixes.length > 0) {
      lines.push("### Bug Fixes");
      lines.push("");
      for (const c of fixes) {
        lines.push(`- ${capitalizeFirst(c.message)} (${c.hash})`);
      }
      lines.push("");
    }
  }

  return lines.join("\n").trim() + "\n";
}

function main() {
  const groups = groupByVersion();
  const changelog = formatChangelog(groups);

  const existing = existsSync(CHANGELOG_PATH)
    ? readFileSync(CHANGELOG_PATH, "utf-8")
    : "";

  if (changelog !== existing) {
    writeFileSync(CHANGELOG_PATH, changelog);
    console.log("CHANGELOG.md updated");
    return true;
  }

  console.log("CHANGELOG.md unchanged");
  return false;
}

main();
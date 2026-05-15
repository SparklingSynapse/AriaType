#!/usr/bin/env node
import { readFileSync } from 'node:fs';
import { execFileSync } from 'node:child_process';

const allowedTypes = new Set([
  'feat',
  'fix',
  'refactor',
  'chore',
  'docs',
  'build',
  'ci',
  'test',
  'perf',
  'style',
]);
const allowedScopes = new Set(['desktop', 'website']);
const headerPattern = /^([a-z]+)(?:\(([a-z-]+)\))?: (.+)$/;
const trailerPattern = /^([A-Za-z][A-Za-z0-9-]*(?: [A-Za-z][A-Za-z0-9-]*)?): (.+)$/u;
const maxHeaderLength = 72;
const maxBodyLineLength = 72;
const skipPatterns = [/^Merge /, /^Revert "/, /^fixup! /, /^squash! /];
const skipStagedScopeCheck = process.env.COMMIT_MSG_LINT_SKIP_STAGED_CHECK === '1';

function fail(errors) {
  console.error('[commit-msg] invalid commit message');
  console.error('');
  for (const error of errors) {
    console.error(`- ${error}`);
  }
  console.error('');
  console.error('Expected format:');
  console.error('  type(scope): subject');
  console.error('');
  console.error('Examples:');
  console.error('  feat(desktop): retry failed transcriptions from history');
  console.error('  chore: add commit message linting');
  console.error('');
  console.error('Footer trailers, when present, must be a single contiguous block.');
  console.error('');
  console.error('See context/spec/commits.md for the full policy.');
  process.exit(1);
}

function readStagedFiles() {
  try {
    const output = execFileSync(
      'git',
      ['diff', '--cached', '--name-only', '--diff-filter=ACMR'],
      { encoding: 'utf8' },
    );

    return output.split('\n').map((line) => line.trim()).filter(Boolean);
  } catch {
    return [];
  }
}

function normalizeMessage(raw) {
  const lines = raw
    .replace(/\r\n/g, '\n')
    .split('\n')
    .filter((line) => !line.startsWith('#'))
    .map((line) => line.replace(/[ \t]+$/u, ''));

  while (lines.length > 0 && lines[lines.length - 1] === '') {
    lines.pop();
  }

  return lines;
}

function firstContentLine(lines) {
  const index = lines.findIndex((line) => line.trim() !== '');
  if (index === -1) {
    return undefined;
  }

  return { index, text: lines[index].trim() };
}

function isTrailerLine(line) {
  return trailerPattern.test(line);
}

function findTrailerStart(lines, headerIndex) {
  let index = lines.length - 1;
  while (index > headerIndex && isTrailerLine(lines[index])) {
    index -= 1;
  }

  const trailerStart = index + 1;
  if (trailerStart >= lines.length) {
    return undefined;
  }

  return trailerStart;
}

function validateAscii(lines, errors) {
  const invalidLine = lines.find((line) => /[^\x09\x20-\x7E]/u.test(line));
  if (invalidLine) {
    errors.push(`message must be English-only ASCII: "${invalidLine}"`);
  }
}

function validateBody(lines, headerIndex, trailerStart, errors) {
  const bodyEnd = trailerStart ?? lines.length;
  const body = lines.slice(headerIndex + 1, bodyEnd);
  const hasBody = body.some((line) => line.trim() !== '');
  if (!hasBody) {
    return;
  }

  if (body[0] !== '') {
    errors.push('separate the subject from the body with a blank line');
  }

  body.forEach((line, offset) => {
    if (isTrailerLine(line)) {
      errors.push('footer trailers must be contiguous at the end of the message');
    }

    if (line.length > maxBodyLineLength && !line.includes('://')) {
      errors.push(`body line ${offset + 1} exceeds ${maxBodyLineLength} characters`);
    }
  });
}

function validateScopeAgainstStagedFiles(scope, errors) {
  if (skipStagedScopeCheck) {
    return;
  }

  const stagedFiles = readStagedFiles();
  if (stagedFiles.length === 0) {
    return;
  }

  const isDesktopPath = (file) => file.startsWith('apps/desktop/');
  const isWebsitePath = (file) => file.startsWith('apps/website/') || file.startsWith('packages/website/');
  const touchesDesktop = stagedFiles.some(isDesktopPath);
  const touchesWebsite = stagedFiles.some(isWebsitePath);

  if (touchesDesktop && touchesWebsite) {
    errors.push('split desktop and website changes into separate commits');
    return;
  }

  if (stagedFiles.every(isDesktopPath) && scope !== 'desktop') {
    errors.push('use scope "desktop" for commits that only touch apps/desktop');
  }

  if (stagedFiles.every(isWebsitePath) && scope !== 'website') {
    errors.push('use scope "website" for commits that only touch the website package');
  }

  if (scope === 'desktop' && !touchesDesktop) {
    errors.push('scope "desktop" requires staged changes under apps/desktop');
  }

  if (scope === 'website' && !touchesWebsite) {
    errors.push('scope "website" requires staged changes under apps/website or packages/website');
  }
}

function validateHeader(header, errors) {
  if (header.length > maxHeaderLength) {
    errors.push(`subject line exceeds ${maxHeaderLength} characters`);
  }

  const match = header.match(headerPattern);
  if (!match) {
    errors.push('subject must match "type(scope): subject" or "type: subject"');
    return undefined;
  }

  const [, type, scope, subject] = match;
  if (!allowedTypes.has(type)) {
    errors.push(`type must be one of: ${Array.from(allowedTypes).join(', ')}`);
  }

  if (scope && !allowedScopes.has(scope)) {
    errors.push('scope must be "desktop", "website", or omitted');
  }

  if (/^[A-Z]/u.test(subject)) {
    errors.push('subject must start with lowercase text');
  }

  if (/[.!?;:]$/u.test(subject)) {
    errors.push('subject must not end with punctuation');
  }

  return { type, scope, subject };
}

function validateTrailers(lines, headerIndex, trailerStart, errors) {
  if (trailerStart === undefined) {
    return;
  }

  if (trailerStart > headerIndex + 1 && lines[trailerStart - 1] !== '') {
    errors.push('separate footer trailers from the body with a blank line');
  }

  for (let index = trailerStart; index < lines.length; index += 1) {
    const line = lines[index];
    const match = line.match(trailerPattern);
    if (!match) {
      errors.push(`invalid trailer line: "${line}"`);
      continue;
    }

    const [, key, value] = match;
    if (key === 'Confidence' && !['low', 'medium', 'high'].includes(value)) {
      errors.push('Confidence trailer must be low, medium, or high');
    }

    if (key === 'Scope-risk' && !['narrow', 'moderate', 'broad'].includes(value)) {
      errors.push('Scope-risk trailer must be narrow, moderate, or broad');
    }
  }
}

const commitMessagePath = process.argv[2];
if (!commitMessagePath) {
  fail(['missing commit message file path']);
}

const lines = normalizeMessage(readFileSync(commitMessagePath, 'utf8'));
const firstLine = firstContentLine(lines);
if (!firstLine) {
  fail(['commit message is empty']);
}

if (skipPatterns.some((pattern) => pattern.test(firstLine.text))) {
  process.exit(0);
}

const errors = [];
validateAscii(lines, errors);
const parsed = validateHeader(firstLine.text, errors);
if (parsed) {
  validateScopeAgainstStagedFiles(parsed.scope, errors);
}
const trailerStart = findTrailerStart(lines, firstLine.index);
validateBody(lines, firstLine.index, trailerStart, errors);
validateTrailers(lines, firstLine.index, trailerStart, errors);

if (errors.length > 0) {
  fail(errors);
}

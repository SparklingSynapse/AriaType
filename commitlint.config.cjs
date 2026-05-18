const allowedTypes = [
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
];

const allowedScopes = ['desktop', 'website'];

module.exports = {
  extends: ['@commitlint/config-conventional'],
  helpUrl: 'context/spec/commits.md',
  ignores: [
    (message) => /^Merge /u.test(message),
    (message) => /^Revert "/u.test(message),
    (message) => /^fixup! /u.test(message),
    (message) => /^squash! /u.test(message),
  ],
  rules: {
    'body-leading-blank': [2, 'always'],
    'body-max-line-length': [2, 'always', 72],
    'footer-leading-blank': [2, 'always'],
    'footer-max-line-length': [0],
    'header-max-length': [2, 'always', 72],
    'scope-case': [2, 'always', 'lower-case'],
    'scope-enum': [2, 'always', allowedScopes],
    'subject-case': [0],
    'subject-empty': [2, 'never'],
    'subject-full-stop': [2, 'never', '.'],
    'type-case': [2, 'always', 'lower-case'],
    'type-empty': [2, 'never'],
    'type-enum': [2, 'always', allowedTypes],
  },
};

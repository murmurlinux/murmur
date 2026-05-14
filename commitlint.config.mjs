export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'type-enum': [
      2,
      'always',
      ['feat', 'fix', 'perf', 'refactor', 'docs', 'chore', 'test', 'ci', 'build', 'style', 'revert'],
    ],
    'subject-case': [2, 'never', ['upper-case', 'pascal-case']],
    'header-max-length': [2, 'always', 100],
    // Dependabot auto-generated bodies contain release-notes URLs that exceed
    // 100 chars; we still enforce header-max-length for the squash-merge subject
    // that release-please reads.
    'body-max-line-length': [0],
  },
};

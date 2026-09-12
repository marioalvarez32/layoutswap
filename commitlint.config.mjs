// Commit subjects follow Conventional Commits (CONTRIBUTING.md): semantic-release
// reads them to pick the next version and write the changelog, so the commit-msg
// hook refuses a subject it could not read. Product names keep their capitals, and
// a subject may run to 100 characters to say what changed in the glossary's words.
export default {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'subject-case': [0],
    'header-max-length': [2, 'always', 100],
    'body-max-line-length': [0],
  },
};

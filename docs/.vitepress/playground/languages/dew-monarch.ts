import type { languages } from 'monaco-editor'

export const dewLanguage: languages.IMonarchLanguage = {
  keywords: ['if', 'then', 'else', 'and', 'or', 'not', 'let', 'in', 'true', 'false'],

  operators: ['+', '-', '*', '/', '^', '%', '<', '<=', '>', '>=', '==', '!='],

  tokenizer: {
    root: [
      // Comments
      [/\/\/.*$/, 'comment'],

      // Keywords
      [/\b(if|then|else|and|or|not|let|in)\b/, 'keyword'],

      // Boolean constants
      [/\b(true|false)\b/, 'constant'],

      // Numbers
      [/\b\d+\.\d*([eE][+-]?\d+)?\b/, 'number.float'],
      [/\b\d*\.\d+([eE][+-]?\d+)?\b/, 'number.float'],
      [/\b\d+[eE][+-]?\d+\b/, 'number.float'],
      [/\b\d+\b/, 'number'],

      // Function calls
      [/\b([a-zA-Z_]\w*)\s*(?=\()/, 'function'],

      // Identifiers (variables)
      [/\b[a-zA-Z_]\w*\b/, 'variable'],

      // Operators
      [/<=|>=|==|!=|<|>/, 'operator.comparison'],
      [/[+\-*\/\^%]/, 'operator'],

      // Punctuation
      [/[(),]/, 'delimiter'],

      // Whitespace
      [/\s+/, 'white'],
    ],
  },
}

export const dewLanguageConfiguration: languages.LanguageConfiguration = {
  comments: {
    lineComment: '//',
  },
  brackets: [['(', ')']],
  autoClosingPairs: [{ open: '(', close: ')' }],
  surroundingPairs: [{ open: '(', close: ')' }],
}

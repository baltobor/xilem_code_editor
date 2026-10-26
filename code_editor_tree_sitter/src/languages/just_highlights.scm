; tree-sitter-just highlights — adapted from the helix flavor
; bundled with tree-sitter-just 0.2.0. Capture names use the
; dotted form (e.g. `keyword.control.import`); tree-sitter-
; highlight collapses them onto the project's flat
; `HIGHLIGHT_NAMES` list via longest-prefix match.

[
  "export"
  "import"
] @keyword.control.import

"mod" @keyword.directive

[
  "alias"
  "set"
  "shell"
] @keyword

[
  "if"
  "else"
] @keyword.control.conditional

(value
  (identifier) @variable)

(alias
  left: (identifier) @variable)

(assignment
  left: (identifier) @variable)

(recipe_header
  name: (identifier) @function)

(dependency
  name: (identifier) @function)

(dependency_expression
  name: (identifier) @function)

(function_call
  name: (identifier) @function)

(parameter
  name: (identifier) @variable.parameter)

(module
  name: (identifier) @namespace)

[
  ":="
  "?"
  "=="
  "!="
  "=~"
  "@"
  "="
  "$"
  "*"
  "+"
  "&&"
  "@-"
  "-@"
  "-"
  "/"
  ":"
] @operator

"," @punctuation.delimiter

[
  "{"
  "}"
  "["
  "]"
  "("
  ")"
  "{{"
  "}}"
] @punctuation.bracket

[ "`" "```" ] @punctuation.special

(boolean) @constant.builtin.boolean

[
  (string)
  (external_command)
] @string

(escape_sequence) @constant.character.escape

(comment) @comment.line

(shebang) @keyword.directive

(setting
  left: (identifier) @keyword
  (#any-of? @keyword
    "allow-duplicate-recipes"
    "allow-duplicate-variables"
    "dotenv-filename"
    "dotenv-load"
    "dotenv-path"
    "dotenv-required"
    "export"
    "fallback"
    "ignore-comments"
    "positional-arguments"
    "shell"
    "shell-interpreter"
    "tempdir"
    "windows-powershell"
    "windows-shell"
    "working-directory"))

(attribute
  (identifier) @attribute
  (#any-of? @attribute
    "confirm"
    "doc"
    "extension"
    "group"
    "linux"
    "macos"
    "metadata"
    "no-cd"
    "no-exit-message"
    "no-quiet"
    "openbsd"
    "parallel"
    "positional-arguments"
    "private"
    "script"
    "unix"
    "windows"
    "working-directory"))

(numeric_error) @error

export def main () { }

# Ensures that a tree-sitter parser is generated for every grammar in the
# working directory. For each grammar, either a `grammar.js` file must exist,
# or a `src/parser.c` must exist.
#
# Grammars names and paths are specified in a `tree-sitter.json`. If
# `tree-sitter.json` does not exist, a single grammar is assumed named `name`,
# in the working directory.
export def 'main generate' (name: string) {
  if not ("tree-sitter.json" | path exists) {
    default-tree-sitter-json $name | save "tree-sitter.json"
  }

  for grammar in (grammars) {
    let grammar_js_path = $grammar.path | path join "grammar.js"
    let parser_c_path = $grammar.path | path join "src/parser.c"
    if ($grammar_js_path | path exists) and not ($parser_c_path | path exists) {
      ^tree-sitter generate $grammar_js_path
    }
  }
}

# Copies files required by `trix-build` to `dir`. A `tree-sitter.json` file in
# the working directory can specify `external-files` for each grammar. These
# files and their parent directories relative to the working directory will be
# reconstructed in `dir`.
#
# The `generate` command must be run before this.
export def 'main install' (dir: path) {
  mkdir $dir
  cp "tree-sitter.json" $dir

  for grammar in (grammars) {
    let grammar_out_path = $dir | path join $grammar.path
    mkdir $grammar_out_path

    let grammar_src = $grammar.path | path join "src"
    cp -r $grammar_src $grammar_out_path

    for external_file in ($grammar.external-files? | default []) {
      let out_dir = $dir | path join ($external_file | path dirname)
      mkdir $out_dir
      cp -r $external_file $out_dir
    }
  }
}

# Writes the grammars referenced in `trix-config` to `dir`, creating it if it
# does not exist.
#
# `config-json` is a JSON mapping grammar names to `{ src, filter }`, where
# `src` is a path to an installed grammar (the output of `trix install`).
#
# For each grammar, its installed tree is copied to `<dir>/<name>`, and a
# `trix-config.json` manifest is written to `<dir>` whose `src` fields are
# relative to `<dir>`. The manifest can later be read by `trix-build` via
# `TrixConfig::from_vendor_dir`.
export def 'main vendor' (--dir: path, --config-json: string) {
  mkdir $dir

  let config = ($config_json | from json)

  let manifest = (
    $config
    | columns
    | reduce --fold {} { |name, acc|
      let grammar = ($config | get $name)
      let dest = ($dir | path join $name)

      # `src` is typically read-only (e.g. from the nix store). Use external
      # `cp` with `--no-preserve=mode` so the vendored copy is writable, which
      # keeps the result usable and re-runnable (otherwise a later `rm`/`cp`
      # over a read-only tree would fail).
      if ($dest | path exists) { ^chmod -R u+w $dest }
      rm -rf $dest
      ^cp -rL --no-preserve=mode $grammar.src $dest

      $acc | insert $name { src: $name, filter: ($grammar.filter? | default null) }
    }
  )

  $manifest | to json | save --force ($dir | path join "trix-config.json")
}

def grammars () {
  "tree-sitter.json" | open | get grammars? | default [{}] | default "." path
}

def default-tree-sitter-json (name: string) {
  {
    name: $name,
    metadata: {
      version: "0.0.0",
    },
  }
}

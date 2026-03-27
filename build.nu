def main (--build: string, --install) {
  if ($build | is-not-empty) {
    build $build
  }
  if $install {
    install
  }
}

def build (name: string) {
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

def install () {
  mkdir $env.out
  cp "tree-sitter.json" $env.out

  for grammar in (grammars) {
    let grammar_out_path = $env.out | path join $grammar.path
    mkdir $grammar_out_path

    let grammar_src = $grammar.path | path join "src"
    cp -r $grammar_src $grammar_out_path

    for external_file in ($grammar.external-files? | default []) {
      let out_dir = $env.out | path join ($external_file | path dirname)
      mkdir $out_dir
      cp -r $external_file $out_dir
    }
  }
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

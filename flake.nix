{
  description = "trix";

  outputs =
    { ... }:
    let
      mkGrammarDrv =
        pkgs: name:
        {
          src,
          filter ? null,
          # Some grammars' `grammar.js` files `require(...)` other tree-sitter
          # grammars (e.g. tree-sitter-cpp requires `tree-sitter-c/grammar`),
          # so `tree-sitter generate` needs their node dependencies present.
          #
          # When `npmDepsHash` is set, the grammar's npm dependencies are
          # fetched via `fetchNpmDeps` (a fixed-output derivation that pins the
          # offline npm cache) and `node_modules` is populated before
          # `tree-sitter generate` runs. This needs only a single hash per
          # grammar, works directly with the upstream `package-lock.json`, and
          # requires no checked-in generated files.
          #
          # To find the hash: set `npmDepsHash` to `pkgs.lib.fakeHash`, build,
          # and copy the `got:` value from the hash-mismatch error. The hash
          # only needs updating when the grammar's `package-lock.json` changes.
          npmDepsHash ? null,
        }:
        let
          npmDeps =
            if npmDepsHash == null then
              { }
            else
              {
                npmDeps = pkgs.fetchNpmDeps {
                  inherit src;
                  hash = npmDepsHash;
                };
                # `npmConfigHook` runs `npm ci --ignore-scripts` during the
                # patch phase to populate `node_modules`. The extra
                # `--ignore-scripts` flag here also makes its `npm rebuild` step
                # skip install scripts, which we don't need (and which often
                # try to compile native code or download binaries from the
                # network, both of which fail in the hermetic build): only the
                # dependencies' `grammar.js` files are required.
                nativeBuildInputs = [ pkgs.npmHooks.npmConfigHook ];
                npmFlags = [ "--ignore-scripts" ];
              };
          drv = pkgs.stdenv.mkDerivation (
            {
              name = "tree-sitter-${name}";
              inherit src;

              nativeBuildInputs = [
                pkgs.nushell
                pkgs.tree-sitter
                pkgs.nodejs_24
              ] ++ (npmDeps.nativeBuildInputs or [ ]);

              configurePhase = ''echo "skipping configure"'';
              buildPhase = ''nu ${./trix.nu} generate "${name}"'';
              installPhase = ''nu ${./trix.nu} install "$out"'';
            }
            // (builtins.removeAttrs npmDeps [ "nativeBuildInputs" ])
          );
        in
        {
          src = drv;
          inherit filter;
        };
      mkLib =
        pkgs: grammars:
        let
          config = builtins.mapAttrs (mkGrammarDrv pkgs) grammars;
          configFile = pkgs.writeText "trix-config.json" (builtins.toJSON config);
        in
        {
          inherit config;
          vendor = pkgs.writeShellScriptBin "trix-vendor" ''
            ${pkgs.nushell}/bin/nu ${./trix.nu} vendor --config-json "$(cat ${configFile})" "$@"
          '';
        };
    in
    {
      inherit mkLib;
    };
}

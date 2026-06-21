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
        }:
        let
          drv = pkgs.stdenv.mkDerivation {
            name = "tree-sitter-${name}";
            inherit src;

            nativeBuildInputs = [
              pkgs.nushell
              pkgs.tree-sitter
              pkgs.nodejs_24
            ];

            configurePhase = ''echo "skipping configure"'';
            buildPhase = ''nu ${./trix.nu} generate "${name}"'';
            installPhase = ''nu ${./trix.nu} install "$out"'';
          };
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

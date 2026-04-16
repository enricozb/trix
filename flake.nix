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
      mkLib = pkgs: grammars: {
        config = builtins.mapAttrs (mkGrammarDrv pkgs) grammars;
        vendor = pkgs.writeShellScriptBin "trix-vendor" ''
          ${pkgs.nu}/bin/nu ${./trix.nu} vendor "$@"
        '';
      };
    in
    {
      inherit mkLib;
    };
}

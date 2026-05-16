{
  ...
}:
{
  projectRootFile = "flake.nix";

  programs = {
    nixfmt.enable = true;
    jsonfmt.enable = true;
    shellcheck.enable = true;
    gofmt.enable = true;
    ruff.enable = true;
    yamlfmt.enable = true;
    toml-sort.enable = true;
    dos2unix.enable = true;
    keep-sorted.enable = true;
    # buggy as of right now
    # nufmt.enable = true;
  };

  settings = {
    # files to exlude from all formatting
    excludes = [ ];
    formatter = {
      # formatter-specific settings
    };
  };
}

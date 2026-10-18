{
  lockFile,
  sources,
  fork,
}:

let
  lock = builtins.fromJSON (builtins.readFile lockFile);

  follow =
    key: path:
    if path == [ ] then
      key
    else
      follow (resolve lock.nodes.${key}.inputs.${builtins.head path}) (builtins.tail path);

  resolve = ref: if builtins.isList ref then follow lock.root ref else ref;

  sourceOf =
    key:
    if key == lock.root then
      { outPath = fork; }
    else
      removeAttrs sources.${key} [ "path" ] // { outPath = sources.${key}.path; };

  nodes = builtins.mapAttrs (
    key: node:
    let
      sourceInfo = sourceOf key;
      inputs = builtins.mapAttrs (_: ref: nodes.${resolve ref}) (node.inputs or { });
      outputs = (import (sourceInfo.outPath + "/flake.nix")).outputs (inputs // { self = flake; });
      flake =
        outputs
        // sourceInfo
        // {
          _type = "flake";
          inherit inputs outputs sourceInfo;
        };
    in
    if node.flake or true then flake else sourceInfo
  ) lock.nodes;
in
nodes.${lock.root}

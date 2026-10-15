{
  lib,
  stdenv,
  fetchFromGitHub,
}:

stdenv.mkDerivation {
  pname = "schbench";
  version = "0-unstable-2025-12-23";

  src = fetchFromGitHub {
    owner = "masoncl";
    repo = "schbench";
    rev = "24e32b8ef67cbafb0761aed68747d90a291767ae";
    hash = "sha256-ItM24mI2a4LcLYDIxPpn1N97mHtTeTmS9B1R6eCMsFg=";
  };

  installPhase = "install -Dm755 schbench $out/bin/schbench";

  meta = {
    description = "Scheduler wakeup and request latency benchmark";
    homepage = "https://github.com/masoncl/schbench";
    license = lib.licenses.gpl2Only;
    mainProgram = "schbench";
  };
}

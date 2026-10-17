fail() {
  echo "FAIL: $*" >&2
  exit 1
}

export HOME=$PWD/home GIT_AUTHOR_NAME=test GIT_AUTHOR_EMAIL=test@example.com GIT_COMMITTER_NAME=test GIT_COMMITTER_EMAIL=test@example.com
mkdir -p "$HOME"
git config --global init.defaultBranch main
git config --global advice.detachedHead false

git init -q --bare fork.git
git init -q --bare mirror.git
mirror=file://$PWD/mirror.git
git clone -q fork.git source 2>/dev/null
cd source

commit() {
  echo "$1" >file
  git add file
  git commit -q -m "$1"
  git rev-parse HEAD
}

a=$(commit A)
b=$(commit B)
git push -q origin main "$a:refs/heads/deploy/example" "$a:refs/heads/deploy/old" "$b:refs/heads/develop"

at() {
  git --git-dir="$1" rev-parse --verify -q "refs/heads/$2" || true
}

checkout() {
  rm -rf ../run
  git clone -q ../fork.git ../run 2>/dev/null
}

checkout
status=0
(cd ../run && MIRROR_TOKEN=s3cr3t-token "$CI_MIRROR" "$mirror") >first.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "the first mirror exits with $status: $(cat first.out)"
[ "$(at ../mirror.git main)" = "$b" ] || fail "main is not mirrored: $(cat first.out)"
[ "$(at ../mirror.git deploy/example)" = "$a" ] || fail "deploy/example is not mirrored: $(cat first.out)"
[ "$(at ../mirror.git deploy/old)" = "$a" ] || fail "deploy/old is not mirrored: $(cat first.out)"
[ -z "$(at ../mirror.git develop)" ] || fail "develop was mirrored although only main and deploy/* are"
grep -q s3cr3t-token first.out && fail "the token shows in the output: $(cat first.out)"

git --git-dir=../mirror.git update-ref refs/heads/deploy/mirror-only "$a"
git --git-dir=../mirror.git update-ref refs/heads/feature "$a"
git push -q origin :deploy/old "+$a:refs/heads/deploy/example" "$b:refs/heads/deploy/server"

checkout
status=0
(cd ../run && "$CI_MIRROR" "$mirror") >second.out 2>&1 || status=$?
[ "$status" -eq 0 ] || fail "the second mirror exits with $status: $(cat second.out)"
[ -z "$(at ../mirror.git deploy/old)" ] || fail "deploy/old deleted at the source is still on the mirror"
[ -z "$(at ../mirror.git deploy/mirror-only)" ] || fail "a deploy branch the source lacks is still on the mirror"
[ "$(at ../mirror.git feature)" = "$a" ] || fail "the prune deleted a branch outside deploy/*"
[ "$(at ../mirror.git deploy/server)" = "$b" ] || fail "the new deploy/server is not mirrored: $(cat second.out)"

git checkout -q --orphan elsewhere
c=$(commit C)
git push -q origin "+$c:refs/heads/deploy/example"
git push -q ../mirror.git "+$c:refs/heads/main"

checkout
status=0
(cd ../run && "$CI_MIRROR" "$mirror") >third.out 2>&1 || status=$?
[ "$status" -ne 0 ] || fail "a main that does not fast-forward on the mirror exits with 0: $(cat third.out)"
[ "$(at ../mirror.git main)" = "$c" ] || fail "main was force-pushed to the mirror"
[ "$(at ../mirror.git deploy/example)" = "$c" ] || fail "deploy/example does not follow the source when it moved back, or a refused main held it: $(cat third.out)"

cat >../capture.py <<'PY'
import http.server, sys

class Capture(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        with open(sys.argv[1], "a") as seen:
            seen.write(f"{self.headers.get('Authorization', '')}\n")
        self.send_response(404)
        self.end_headers()

    def log_message(self, *args):
        pass

server = http.server.HTTPServer(("127.0.0.1", 0), Capture)
with open(sys.argv[2], "w") as port:
    port.write(str(server.server_port))
server.serve_forever()
PY
python3 ../capture.py ../headers ../port &
capture=$!
for _ in $(seq 50); do
  [ -s ../port ] && break
  sleep 0.1
done
[ -s ../port ] || fail "the capture server did not start"
http_mirror=http://127.0.0.1:$(cat ../port)/alice/nixos.git

(cd ../run && MIRROR_TOKEN=s3cr3t-token timeout 30 "$CI_MIRROR" "$http_mirror") >http.out 2>&1 || true
expected="Basic $(printf '%s' 'x-access-token:s3cr3t-token' | base64 -w0)"
grep -qxF "$expected" ../headers || fail "the mirror did not get the header $expected: $(cat ../headers)"
grep -q s3cr3t-token http.out && fail "the token shows in the output: $(cat http.out)"

: >../headers
(cd ../run && timeout 30 "$CI_MIRROR" "$http_mirror") >anonymous.out 2>&1 || true
[ -s ../headers ] || fail "the mirror got no request without a token"
grep -qv '^$' ../headers && fail "a header was sent without MIRROR_TOKEN: $(cat ../headers)"
kill "$capture"

status=0
(cd ../run && "$CI_MIRROR") >usage.out 2>&1 || status=$?
[ "$status" -eq 2 ] || fail "no URL exits with $status instead of 2"

echo "ci-mirror: all scenarios passed"

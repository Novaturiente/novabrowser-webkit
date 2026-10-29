#!/usr/bin/env bash
# Phase 10 checks with the `pass` backend on a throwaway GnuPG home and password store,
# against a local HTTPS server with a self-signed certificate (fill never runs on http).
# Needs: pass, gpg, openssl, python3, a Wayland/X11 session.
# Usage: tests/passwords.sh [path/to/novabrowser-webkit]
set -u
. "$(dirname "$0")/lib.sh"
for c in pass gpg openssl; do command -v $c >/dev/null || { echo "skip: no $c"; exit 0; }; done

# vault: unprotected test key (no pinentry), one login for 127.0.0.1
export GNUPGHOME=$tmp/gnupg PASSWORD_STORE_DIR=$tmp/store
mkdir -m 700 $GNUPGHOME
gpg --batch --pinentry-mode loopback --passphrase '' --quick-gen-key 'nova test <test@nova.invalid>' default default never 2>/dev/null
key=$(gpg --list-keys --with-colons 2>/dev/null | awk -F: '/^fpr/ {print $10; exit}')
pass init "$key" >/dev/null
printf 'hunter2\n' | pass insert -m 127.0.0.1/alice >/dev/null
trap '[ -n "$pid" ] && kill $pid 2>/dev/null; [ -n "${http:-}" ] && kill $http 2>/dev/null; gpgconf --kill gpg-agent; rm -rf "$tmp"' EXIT

# HTTPS server on 127.0.0.1 with a throwaway self-signed certificate
srv=$tmp/www; mkdir -p $srv
cp "$(dirname "$0")/pages/login.html" "$(dirname "$0")/pages/signup.html" $srv/; echo '<title>done</title>' > $srv/done.html
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj /CN=127.0.0.1 -addext subjectAltName=IP:127.0.0.1 \
    -keyout $tmp/key.pem -out $tmp/cert.pem 2>/dev/null
port=$((20000 + RANDOM % 20000))
(cd $srv && exec python3 -c "
import http.server, ssl
s = http.server.HTTPServer(('127.0.0.1', $port), http.server.SimpleHTTPRequestHandler)
c = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER); c.load_cert_chain('$tmp/cert.pem', '$tmp/key.pem')
s.socket = c.wrap_socket(s.socket, server_side=True)
s.serve_forever()" 2>$tmp/http.log) &
http=$!
sleep 0.5
base=https://127.0.0.1:$port

start "$(page start)"
wait_for '"title":"start"' 30 || fail "start"
"$bin" --cmd "set passwords.backend pass" >/dev/null

# self-signed: the TLS prompt; y allows this certificate for the host and reloads
"$bin" --cmd "open $base/login.html" >/dev/null
wait_for '"prompt":"[^"]*(certificate|TLS|secure)' 10 && ok "TLS error asks in the prompt" || fail "tls prompt: $(q)"
sleep 1   # past the prompt's grace period
"$bin" --key y >/dev/null
wait_for '"title":"user= pass="' 10 && ok "y allows the certificate, page loads" || fail "tls allow: $(q)"

# 10.4 password-fill: one match fills user + password
"$bin" --cmd "password-fill" >/dev/null
wait_for '"title":"user=alice pass=hunter2"' 15 && ok "10.4 password-fill fills user and password (pass)" || fail "10.4 fill: $(q)"

# 10.1 no match on another site
"$bin" --cmd "open https://localhost:$port/login.html" >/dev/null
wait_for '"prompt":"' 10 && { sleep 1; "$bin" --key y >/dev/null; }
wait_for '"title":"user= pass="' 10
"$bin" --cmd "password-fill" >/dev/null
wait_for 'no pass login for localhost' 15 && ok "10.1 no login for another host" || fail "10.1 none: $(q)"

# 10.4 never on http
"$bin" --cmd "open http://127.0.0.1:1/x" >/dev/null; sleep 1
"$bin" --cmd "password-fill" >/dev/null
wait_for 'not filling a login over plain http' 5 && ok "10.4 refuses plain http" || fail "10.4 http: $(q)"

# 10.6 save after submit: prompt, y, entry lands in the store
"$bin" --cmd "open $base/login.html?submit=bob,s3cret" >/dev/null
wait_for '"prompt":"Save this login for bob on 127.0.0.1 to pass' 15 && ok "10.6 save prompt after submit" || fail "10.6 prompt: $(q)"
sleep 1
"$bin" --key y >/dev/null
for _ in $(seq 50); do [ -e "$PASSWORD_STORE_DIR/127.0.0.1/bob.gpg" ] && break; sleep 0.2; done
[ "$(pass show 127.0.0.1/bob 2>/dev/null | head -1)" = s3cret ] && ok "10.6 y saves the login to pass" || fail "10.6 save: $(find $PASSWORD_STORE_DIR -name '*.gpg') $(q)"

# 10.6 update: same user, new password
"$bin" --cmd "open $base/login.html?submit=bob,n3wpass" >/dev/null
wait_for '"prompt":"Update the pass password for bob' 15 && ok "10.6 update prompt for a changed password" || fail "10.6 update prompt: $(q)"
sleep 1
"$bin" --key y >/dev/null
for _ in $(seq 50); do [ "$(pass show 127.0.0.1/bob 2>/dev/null | head -1)" = n3wpass ] && break; sleep 0.2; done
[ "$(pass show 127.0.0.1/bob 2>/dev/null | head -1)" = n3wpass ] && ok "10.6 y updates the password" || fail "10.6 update: $(pass show 127.0.0.1/bob 2>&1 | head -1)"

# 10.6 same password again: no prompt
"$bin" --cmd "open $base/login.html?submit=bob,n3wpass" >/dev/null
wait_for '"title":"done"' 15; sleep 2
q | grep -q '"prompt":null' && ok "10.6 known password: no prompt" || fail "10.6 repeat: $(q)"

# 10.4 several matches: the list; Enter picks the first
"$bin" --cmd "open $base/login.html" >/dev/null
wait_for '"title":"user= pass="' 10
"$bin" --cmd "password-fill" >/dev/null; sleep 2
"$bin" --key '<Return>' >/dev/null
wait_for '"title":"user=(alice|bob) pass=(hunter2|n3wpass)"' 15 && ok "10.3 several matches: list, Enter fills one" || fail "10.3 list: $(q)"

# 10.6 split login: page 1 submits only the user, page 2 only the password
"$bin" --cmd "open $base/login.html?submit=carol," >/dev/null
wait_for '"title":"done"' 15
"$bin" --cmd "open $base/login.html?submit=,spl1t" >/dev/null
wait_for '"prompt":"Save this login for carol on 127.0.0.1' 15 && ok "10.6 split login remembers page 1's user" || fail "10.6 split: $(q)"
"$bin" --key '<Escape>' >/dev/null; sleep 0.5

# 10.2/10.3 focusing a login field shows the list (never preselected); Down Enter fills
"$bin" --cmd "open $base/login.html?x=2" >/dev/null   # autofocus on the user field
wait_for '"suggestions":2' 15 && ok "10.2 focused login field shows 2 suggestions" || fail "10.2 list: $(q)"
"$bin" --key '<Down><Return>' >/dev/null
wait_for '"title":"user=alice pass=hunter2"' 15 && ok "10.3 Down, Enter fills the first row" || fail "10.3 keys: $(q)"

# 10.5 sign-up field: "Generate a strong password" row fills 20 characters
"$bin" --cmd "open $base/signup.html" >/dev/null
wait_for '"suggestions":[1-9]' 15 && ok "10.5 new-password field shows the list" || fail "10.5 generate row: $(q)"
"$bin" --key '<Up><Return>' >/dev/null   # Up from nothing = the last row, "Generate"
wait_for '"title":"new=20"' 10 && ok "10.5 generated password is 20 characters" || fail "10.5 generate: $(q)"

stop
finish
